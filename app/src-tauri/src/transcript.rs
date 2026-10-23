//! The conversation on disk (knowledge/11, "Mobile non-negotiables"): what the
//! user said to Verdure and what it answered, kept in the app's private folder
//! so that leaving the chat screen, switching apps, or the OS killing the app
//! loses nothing. The Rust core owns it — a reply generated while the webview
//! was suspended is still recorded, and the screen re-renders from this file
//! whenever it comes back. One JSON line per entry, appended, never rewritten
//! mid-conversation; compacted oldest-first when it grows past `KEEP` entries.
//!
//! This is also where the chat's memory (knowledge/22) now comes from: the
//! completed exchanges in this file are the turns the model is shown.

use std::path::Path;

use crate::{llm, web};

/// The file's name, beside the models and the context store. Excluded from
/// cloud backup by the Android build (`patch-android-project.py`).
pub(crate) const FILE: &str = "chat.jsonl";
/// Entries kept when the file is compacted. Four hundred is weeks of daily
/// use, and far more than any model window will be shown.
const KEEP: usize = 400;
/// Compact when the file holds this many entries — twice `KEEP`, so a
/// rewrite happens rarely rather than on every append past the limit.
const COMPACT_AT: usize = KEEP * 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Role {
    User,
    Assistant,
}

/// What a reply was grounded on: a web search or the user's own records
/// (knowledge/24, 25) — kept so the sources footer survives a re-render.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebRecord {
    pub query: String,
    #[serde(default)]
    pub sources: Vec<web::Snippet>,
    #[serde(default)]
    pub unavailable: Option<web::UnavailableReason>,
    #[serde(default)]
    pub memory_label: Option<String>,
}

/// One line of the transcript.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Entry {
    pub role: Role,
    pub text: String,
    /// Unix milliseconds.
    pub at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web: Option<WebRecord>,
}

/// The conversation, oldest first — at most `KEEP` entries. A line that
/// cannot be read (a partial last line from a kill mid-write) is skipped, not
/// fatal: one bad line must never hide the rest of the conversation.
pub(crate) fn load(dir: &Path) -> Vec<Entry> {
    let text = std::fs::read_to_string(dir.join(FILE)).unwrap_or_default();
    let mut entries: Vec<Entry> = text.lines().filter_map(|l| serde_json::from_str(l.trim()).ok()).collect();
    if entries.len() > KEEP {
        entries.drain(..entries.len() - KEEP);
    }
    entries
}

/// Add one entry to the end. Compacts the file to the newest `KEEP` entries
/// once it holds `COMPACT_AT`, so it stays small on a phone.
pub(crate) fn append(dir: &Path, entry: &Entry) -> Result<(), String> {
    use std::io::Write;
    let path = dir.join(FILE);
    let line = serde_json::to_string(entry).map_err(|e| format!("couldn't save the conversation: {e}"))?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("couldn't save the conversation: {e}"))?;
    writeln!(f, "{line}").map_err(|e| format!("couldn't save the conversation: {e}"))?;
    drop(f);
    let count = std::fs::read_to_string(&path).map(|t| t.lines().count()).unwrap_or(0);
    if count >= COMPACT_AT {
        let kept = load(dir);
        let body = kept
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .map(|l| l + "\n")
            .collect::<String>();
        let tmp = dir.join(format!("{FILE}.tmp"));
        std::fs::write(&tmp, body).map_err(|e| format!("couldn't tidy the conversation: {e}"))?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("couldn't tidy the conversation: {e}"))?;
    }
    Ok(())
}

/// Start again: the file goes, so does every entry.
pub(crate) fn clear(dir: &Path) -> Result<(), String> {
    match std::fs::remove_file(dir.join(FILE)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("couldn't clear the conversation: {e}")),
    }
}

/// The completed exchanges, as the model is shown them (knowledge/22). A
/// question with no answer after it is left out: shown to the model, it would
/// teach that questions go unanswered.
pub(crate) fn history(entries: &[Entry]) -> Vec<llm::Turn> {
    let mut turns = Vec::new();
    let mut i = 0;
    while i < entries.len() {
        if entries[i].role == Role::User {
            if let Some(next) = entries.get(i + 1) {
                if next.role == Role::Assistant {
                    turns.push(llm::Turn { user: entries[i].text.clone(), assistant: next.text.clone() });
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    turns
}

/// Whether `message` is already the last entry, waiting for its answer — a
/// retry (a resumed download, the second ask after a web search) must not
/// write the question twice.
pub(crate) fn awaiting(entries: &[Entry], message: &str) -> bool {
    matches!(entries.last(), Some(e) if e.role == Role::User && e.text == message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("verdure-transcript-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn say(role: Role, text: &str) -> Entry {
        Entry { role, text: text.to_string(), at_ms: 1_000, web: None }
    }

    #[test]
    fn entries_round_trip_and_a_torn_line_is_skipped() {
        let d = dir("roundtrip");
        assert!(load(&d).is_empty(), "no file is an empty conversation, not an error");
        append(&d, &say(Role::User, "hello")).unwrap();
        let grounded = Entry {
            role: Role::Assistant,
            text: "hi [1]".to_string(),
            at_ms: 2_000,
            web: Some(WebRecord {
                query: "hello".to_string(),
                sources: vec![web::Snippet {
                    title: "T".to_string(),
                    url: "https://example.org/".to_string(),
                    domain: "example.org".to_string(),
                    date: None,
                    text: "…".to_string(),
                }],
                unavailable: None,
                memory_label: None,
            }),
        };
        append(&d, &grounded).unwrap();
        // The app was killed mid-write: a partial last line.
        use std::io::Write;
        write!(std::fs::OpenOptions::new().append(true).open(d.join(FILE)).unwrap(), "{{\"role\":\"user\",\"te").unwrap();
        let got = load(&d);
        assert_eq!(got, vec![say(Role::User, "hello"), grounded]);
        clear(&d).unwrap();
        assert!(load(&d).is_empty());
        clear(&d).unwrap(); // twice is fine
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn history_is_only_the_completed_exchanges() {
        let entries = vec![
            say(Role::User, "one"),
            say(Role::Assistant, "1"),
            say(Role::User, "lost"), // no answer: the app died
            say(Role::User, "two"),
            say(Role::Assistant, "2"),
            say(Role::User, "pending"),
        ];
        let h = history(&entries);
        assert_eq!(h.len(), 2);
        assert_eq!((h[0].user.as_str(), h[0].assistant.as_str()), ("one", "1"));
        assert_eq!((h[1].user.as_str(), h[1].assistant.as_str()), ("two", "2"));
        assert!(awaiting(&entries, "pending"));
        assert!(!awaiting(&entries, "two"));
        assert!(!awaiting(&entries[..2], "one"), "answered questions are not awaiting");
    }

    #[test]
    fn the_file_is_compacted_to_the_newest_entries() {
        let d = dir("compact");
        for i in 0..COMPACT_AT {
            append(&d, &say(Role::User, &format!("m{i}"))).unwrap();
        }
        let lines = std::fs::read_to_string(d.join(FILE)).unwrap().lines().count();
        assert_eq!(lines, KEEP, "compacted on reaching the ceiling");
        let got = load(&d);
        assert_eq!(got.len(), KEEP);
        assert_eq!(got[0].text, format!("m{}", COMPACT_AT - KEEP), "the oldest went, the newest stayed");
        assert_eq!(got.last().unwrap().text, format!("m{}", COMPACT_AT - 1));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
