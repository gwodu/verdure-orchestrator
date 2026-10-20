//! Web search: the one thing Verdure sends off the device (knowledge/24).
//!
//! The shape of the promise, which every function here serves: **only the
//! search words the user approved leave the phone** — never the conversation,
//! never a name or number that slipped into the question, never the device's
//! identity beyond its IP. The words go to Verdure's relay (`RELAY_URL`), which
//! forwards them to the search provider and keeps no record; the reply is a
//! handful of short snippets the local model answers from.
//!
//! What lives here is the pure core of that: scrubbing the query, sizing the
//! snippets to the device's context window, rendering them into the prompt, and
//! reading the network state the UI gates on. The one network call (`search`)
//! is a thin function at the bottom. There is deliberately no trait yet: one
//! relay, one provider behind it; the seam gets a trait with its second adapter
//! (the offline brief pack, knowledge/24 phase 2), not before.

use std::io::Read;
use std::time::Duration;

/// Where the approved query goes. One hostname, compiled in — the same address
/// the desktop updater already trusts (knowledge/23). `VERDURE_SEARCH_RELAY` at
/// build time overrides it for a developer pointing a local build at a dev relay.
pub(crate) const RELAY_URL: &str = match option_env!("VERDURE_SEARCH_RELAY") {
    Some(url) => url,
    None => "https://verdureai.com/api/search",
};

/// A fixed, generic User-Agent: it identifies the app's major version and
/// nothing about the device, OS or person (knowledge/24, "what leaves the device").
const USER_AGENT: &str = "Verdure/1";

/// Longest query we will send. Long enough for a real question, short enough that
/// a pasted document or conversation cannot ride along inside it.
const MAX_QUERY_CHARS: usize = 200;

/// Hard cap on what we will read back from the relay. The relay already keeps
/// replies to a few KB; this is the belt to its braces, so a misbehaving relay
/// can't make the phone download megabytes on mobile data.
const MAX_RESPONSE_BYTES: u64 = 20 * 1024;

/// Snippets never take more than a third of the window, and never more than this
/// many tokens — small models answer worse from long context, not better
/// (knowledge/24, "few, short, high-quality").
const MAX_SNIPPET_TOKENS: u32 = 600;
const MIN_SNIPPET_TOKENS: u32 = 120;
/// At most this many sources per answer. Three to five is where small models
/// still read every one; the relay returns no more than this either.
pub(crate) const MAX_SOURCES: usize = 5;
/// Rough chars-per-token for sizing BEFORE the model is loaded. Conservative on
/// purpose: an overestimate of token cost only trims a snippet a little short;
/// an underestimate would crowd the user's question out of the window.
const CHARS_PER_TOKEN: usize = 3;

/// One search result as the relay returns it and the prompt consumes it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snippet {
    pub title: String,
    pub url: String,
    pub domain: String,
    #[serde(default)]
    pub date: Option<String>,
    pub text: String,
}

/// Can the phone reach the relay, and over what? The UI's data-use rule turns
/// on this (knowledge/24): search on Wi-Fi by default, ask before mobile data,
/// and tell the model honestly when there is no connection at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum NetState {
    Offline,
    Wifi,
    Cellular,
    /// The platform can't tell (Android today). The UI treats this like
    /// cellular — asking is the honest default when we don't know.
    Unknown,
}

/// Turn what the user typed into the words that may leave the device.
///
/// Removes the things people put in a chat message without meaning to publish
/// them — email addresses and anything that looks like a phone, card or ID
/// number — then collapses whitespace and caps the length. The user sees and
/// approves this exact string before it is sent, so the scrub is a floor, not
/// the whole defence: names and places stay, because "weather in Ibadan" needs
/// Ibadan. (Stripping names from a contact list is phase 2, knowledge/24.)
pub(crate) fn prepare_query(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    for word in message.split_whitespace() {
        // Judge the token without the punctuation that trails it in a sentence
        // ("+234-803-123-4567," is still a phone number), then drop the whole
        // token — the comma carried no meaning without its number.
        let bare = word.trim_end_matches([',', '.', ';', ':', '!', '?', ')']);
        if looks_like_email(bare) || looks_like_number_string(bare) {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    truncate_chars(&out, MAX_QUERY_CHARS)
}

fn looks_like_email(word: &str) -> bool {
    let Some(at) = word.find('@') else { return false };
    at > 0 && word[at + 1..].contains('.')
}

/// Seven or more digits in one token, allowing the separators people type in
/// numbers (+234-803-123-4567, 4111 is too short to matter, 1234567890 isn't).
fn looks_like_number_string(word: &str) -> bool {
    let digits = word.chars().filter(char::is_ascii_digit).count();
    let only_numberish = word
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '(' | ')' | '.' | '/'));
    digits >= 7 && only_numberish
}

fn truncate_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => s[..i].trim_end().to_string(),
        None => s.to_string(),
    }
}

/// How many tokens of snippets this window can carry and still leave the model
/// room to think and answer. The reply budget and the instruction are fixed
/// costs (llm.rs); history is what gives way when sources are present.
pub(crate) fn snippet_budget_tokens(n_ctx: u32) -> u32 {
    (n_ctx / 3).clamp(MIN_SNIPPET_TOKENS, MAX_SNIPPET_TOKENS)
}

/// Keep the snippets that fit `budget_tokens`, in rank order, trimming the one
/// that straddles the limit rather than dropping it: a shortened top result is
/// worth more than a whole fifth one. Returns at most `max_count`.
pub(crate) fn fit_snippets(snippets: Vec<Snippet>, budget_tokens: u32, max_count: usize) -> Vec<Snippet> {
    let mut remaining = budget_tokens as usize * CHARS_PER_TOKEN;
    let mut kept = Vec::new();
    for mut s in snippets.into_iter().take(max_count) {
        // The header line ([n] domain — title) costs too.
        let header = s.domain.len() + s.title.len() + 8;
        if remaining <= header + 40 {
            break; // not enough left for a useful sentence
        }
        remaining -= header;
        if s.text.len() > remaining {
            s.text = truncate_chars(&s.text, remaining);
        }
        remaining = remaining.saturating_sub(s.text.len());
        kept.push(s);
    }
    kept
}

/// The sources, as the model reads them — placed in the final user turn ahead
/// of the question (llm.rs). Short on purpose: every instruction token competes
/// with the question for a small model's attention (see `llm::SYSTEM`'s note).
/// Citations are plain `[n]`; the UI turns them into the source list.
pub(crate) fn sources_block(query: &str, snippets: &[Snippet]) -> String {
    let mut b = format!("Web search results for \"{query}\" (just now):\n");
    for (i, s) in snippets.iter().enumerate() {
        b.push_str(&format!("[{}] {}", i + 1, s.domain));
        if let Some(d) = s.date.as_deref().filter(|d| !d.is_empty()) {
            b.push_str(&format!(", {d}"));
        }
        b.push_str(&format!(" — {}\n{}\n", s.title, s.text.trim()));
    }
    b.push_str(
        "Answer the question below from these results, and put [1], [2]… after facts you take from them. \
         If the results don't answer it, say so, then answer from what you know.",
    );
    b
}

/// What the model is told when the user wanted the web checked and it couldn't
/// be. Honesty is the product here: a confident answer that quietly skipped
/// the check is exactly the failure the feature exists to prevent.
pub(crate) fn unavailable_note() -> &'static str {
    "(The web couldn't be checked for this — the device is offline. \
     Answer from what you know, and say that it wasn't checked.)"
}

/// The relay's reply. `query` echoes what was searched so the UI can show the
/// user the exact words, not the ones it thinks it sent.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelayReply {
    #[serde(default)]
    results: Vec<Snippet>,
}

/// Send `query` to the relay; back come ranked snippets. The request carries the
/// query, a token budget (so the relay trims server-side) and nothing else — no
/// cookies, no location, a fixed User-Agent. Errors are already in the user's
/// words; the technical cause goes to the log.
pub(crate) fn search(relay_url: &str, query: &str, max_tokens: u32) -> Result<Vec<Snippet>, String> {
    let body = serde_json::json!({ "q": query, "maxTokens": max_tokens }).to_string();
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(12))
        .user_agent(USER_AGENT)
        .build();
    let response = agent
        .post(relay_url)
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .send_string(&body)
        .map_err(|e| {
            eprintln!("web search relay error: {e}");
            "Couldn't reach the web right now. Check your connection and try again.".to_string()
        })?;
    let mut raw = String::new();
    response
        .into_reader()
        .take(MAX_RESPONSE_BYTES)
        .read_to_string(&mut raw)
        .map_err(|e| {
            eprintln!("web search relay read error: {e}");
            "The web search reply couldn't be read. Please try again.".to_string()
        })?;
    parse_reply(&raw)
}

/// Pure half of `search`, so the parsing is unit-tested without a server.
fn parse_reply(raw: &str) -> Result<Vec<Snippet>, String> {
    let reply: RelayReply = serde_json::from_str(raw).map_err(|e| {
        eprintln!("web search relay returned something unexpected: {e}");
        "The web search reply couldn't be read. Please try again.".to_string()
    })?;
    Ok(reply
        .results
        .into_iter()
        .filter(|s| !s.text.trim().is_empty())
        .collect())
}

/// Reachability of the relay host, as the OS reports it. Apple platforms answer
/// through SystemConfiguration (an audited wrapper crate, not hand-written FFI —
/// skills/testing.md); `IS_WWAN` is how iOS says "cellular". macOS never sets it,
/// so a Mac on a phone hotspot reads as Wi-Fi — acceptable, since the Wi-Fi-only
/// rule exists for phone data plans (knowledge/24). Elsewhere: `Unknown`.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) fn network_state() -> NetState {
    use system_configuration::network_reachability::{ReachabilityFlags, SCNetworkReachability};
    let host = match std::ffi::CString::new(relay_host()) {
        Ok(h) => h,
        Err(_) => return NetState::Unknown,
    };
    let Some(r) = SCNetworkReachability::from_host(&host) else {
        return NetState::Unknown;
    };
    match r.reachability() {
        Ok(f) => net_state_from(
            f.contains(ReachabilityFlags::REACHABLE),
            f.contains(ReachabilityFlags::CONNECTION_REQUIRED),
            f.contains(ReachabilityFlags::IS_WWAN),
        ),
        Err(_) => NetState::Unknown,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(crate) fn network_state() -> NetState {
    NetState::Unknown
}

/// The flags → state rule, kept pure so it is tested on every platform.
/// "Reachable but a connection is required" (no interface up yet) is offline
/// for our purposes: a search would hang, not succeed.
pub(crate) fn net_state_from(reachable: bool, connection_required: bool, wwan: bool) -> NetState {
    if !reachable || connection_required {
        NetState::Offline
    } else if wwan {
        NetState::Cellular
    } else {
        NetState::Wifi
    }
}

/// The relay's hostname, for reachability checks. Derived from `RELAY_URL` so a
/// build-time override is honoured everywhere.
#[cfg_attr(not(any(target_os = "macos", target_os = "ios")), allow(dead_code))]
fn relay_host() -> &'static str {
    let rest = RELAY_URL.split("://").nth(1).unwrap_or(RELAY_URL);
    rest.split('/').next().unwrap_or(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(domain: &str, text: &str) -> Snippet {
        Snippet {
            title: format!("Title from {domain}"),
            url: format!("https://{domain}/x"),
            domain: domain.to_string(),
            date: None,
            text: text.to_string(),
        }
    }

    #[test]
    fn scrub_drops_emails_and_long_numbers_but_keeps_the_question() {
        let q = prepare_query("my sister ada@example.com in Ibadan, call +234-803-123-4567, naira rate today");
        assert_eq!(q, "my sister in Ibadan, call naira rate today");
    }

    #[test]
    fn scrub_keeps_short_numbers_people_search_for() {
        // Years, prices and model numbers are the search, not a secret.
        assert_eq!(prepare_query("iPhone 15 price 2026"), "iPhone 15 price 2026");
        assert_eq!(prepare_query("AFCON 2025 final score"), "AFCON 2025 final score");
    }

    #[test]
    fn scrub_caps_the_length_and_collapses_whitespace() {
        let long = "word ".repeat(100);
        let q = prepare_query(&long);
        assert!(q.chars().count() <= MAX_QUERY_CHARS);
        assert!(!q.contains("  "));
        assert!(!q.ends_with(' '));
    }

    #[test]
    fn scrub_handles_multibyte_text_at_the_cap() {
        let q = prepare_query(&"é".repeat(300));
        assert_eq!(q.chars().count(), MAX_QUERY_CHARS);
    }

    #[test]
    fn budget_is_a_third_of_the_window_within_bounds() {
        assert_eq!(snippet_budget_tokens(512), MIN_SNIPPET_TOKENS.max(512 / 3));
        assert_eq!(snippet_budget_tokens(1024), 341);
        assert_eq!(snippet_budget_tokens(4096), MAX_SNIPPET_TOKENS);
    }

    #[test]
    fn fit_keeps_rank_order_and_trims_the_straddling_snippet() {
        let long = "a".repeat(2000);
        let got = fit_snippets(
            vec![snippet("one.com", &long), snippet("two.com", &long), snippet("three.com", &long)],
            200, // 600 chars
            5,
        );
        assert_eq!(got.len(), 1, "a 2000-char first snippet uses the whole budget");
        assert_eq!(got[0].domain, "one.com");
        assert!(got[0].text.len() <= 600);
    }

    #[test]
    fn fit_respects_the_count_cap_and_skips_nothing_when_all_fit() {
        let got = fit_snippets(
            (0..7).map(|i| snippet(&format!("s{i}.com"), "short text")).collect(),
            600,
            5,
        );
        assert_eq!(got.iter().map(|s| s.domain.as_str()).collect::<Vec<_>>(), ["s0.com", "s1.com", "s2.com", "s3.com", "s4.com"]);
    }

    #[test]
    fn sources_block_numbers_and_dates_each_source() {
        let mut a = snippet("cbn.gov.ng", "The rate is 1500.");
        a.date = Some("2026-10-19".into());
        let b = snippet("news.example", "Markets moved.");
        let block = sources_block("naira rate", &[a, b]);
        assert!(block.starts_with("Web search results for \"naira rate\""));
        assert!(block.contains("[1] cbn.gov.ng, 2026-10-19 — Title from cbn.gov.ng\nThe rate is 1500."));
        assert!(block.contains("[2] news.example — Title from news.example\nMarkets moved."));
        assert!(block.contains("put [1], [2]"));
    }

    #[test]
    fn parses_the_relay_reply_and_drops_empty_snippets() {
        let raw = r#"{"query":"x","results":[
            {"title":"T","url":"https://a.com/p","domain":"a.com","date":"2026-10-01","text":"Hello"},
            {"title":"Empty","url":"https://b.com","domain":"b.com","text":"   "}
        ]}"#;
        let got = parse_reply(raw).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].domain, "a.com");
        assert_eq!(got[0].date.as_deref(), Some("2026-10-01"));
    }

    #[test]
    fn a_broken_reply_is_an_error_in_the_users_words() {
        let err = parse_reply("<html>502</html>").unwrap_err();
        assert!(!err.contains("expected"), "no serde jargon on screen: {err}");
        assert!(err.contains("try again"));
    }

    #[test]
    fn network_state_rule() {
        assert_eq!(net_state_from(false, false, false), NetState::Offline);
        assert_eq!(net_state_from(true, true, false), NetState::Offline);
        assert_eq!(net_state_from(true, false, true), NetState::Cellular);
        assert_eq!(net_state_from(true, false, false), NetState::Wifi);
    }

    #[test]
    fn relay_host_is_the_bare_hostname() {
        assert_eq!(relay_host(), RELAY_URL.split("://").nth(1).unwrap().split('/').next().unwrap());
        assert!(!relay_host().contains('/'));
    }
}
