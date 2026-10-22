//! Text-side helpers that mirror what scribe APIs offer (Corti's Keyterms and
//! Replacements, knowledge/26 Part 1) so an integrator maps their config
//! one-to-one, plus the cleanup of engine artefacts.

/// Whisper's bias prompt: the keyterms as a comma list. Whisper treats the
/// initial prompt as prior context, so spelling drug and place names here
/// raises the odds they come out spelled that way.
pub fn keyterms_prompt(keyterms: &[String]) -> Option<String> {
    let terms: Vec<&str> = keyterms
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    (!terms.is_empty()).then(|| terms.join(", "))
}

/// Apply exact, case-insensitive, whole-word replacements in order
/// ("B P" → "BP", "para" → "paracetamol"). Multi-word sources are matched
/// across single spaces.
pub fn apply_replacements(text: &str, replacements: &[(String, String)]) -> String {
    let mut out = text.to_string();
    for (from, to) in replacements {
        let from = from.trim();
        if from.is_empty() {
            continue;
        }
        out = replace_whole_words(&out, from, to);
    }
    out
}

fn replace_whole_words(text: &str, from: &str, to: &str) -> String {
    let lower = text.to_lowercase();
    let needle = from.to_lowercase();
    let mut result = String::with_capacity(text.len());
    let mut last = 0usize;
    let mut search = 0usize;
    while let Some(pos) = lower[search..].find(&needle) {
        let start = search + pos;
        let end = start + needle.len();
        let before_ok = start == 0
            || !lower[..start]
                .chars()
                .next_back()
                .map(char::is_alphanumeric)
                .unwrap_or(false);
        let after_ok = end == lower.len()
            || !lower[end..]
                .chars()
                .next()
                .map(char::is_alphanumeric)
                .unwrap_or(false);
        if before_ok && after_ok {
            result.push_str(&text[last..start]);
            result.push_str(to);
            last = end;
        }
        search = end.max(start + 1);
        if search > lower.len() {
            break;
        }
    }
    result.push_str(&text[last..]);
    result
}

/// Drop the non-speech markers Whisper writes for silence, music and noise
/// (`[BLANK_AUDIO]`, `(music)`, `*laughs*`, `♪`), collapse whitespace.
pub fn clean_transcript(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth_sq = 0i32;
    let mut depth_par = 0i32;
    let mut in_star = false;
    for c in text.chars() {
        match c {
            '[' => depth_sq += 1,
            ']' if depth_sq > 0 => depth_sq -= 1,
            '(' => depth_par += 1,
            ')' if depth_par > 0 => depth_par -= 1,
            '*' => in_star = !in_star,
            '♪' | '♫' => {}
            c if depth_sq == 0 && depth_par == 0 && !in_star => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyterms_become_a_comma_list_or_nothing() {
        assert_eq!(
            keyterms_prompt(&["amoxicillin".into(), " Ikorodu ".into(), "".into()]).as_deref(),
            Some("amoxicillin, Ikorodu")
        );
        assert_eq!(keyterms_prompt(&[]), None);
        assert_eq!(keyterms_prompt(&["  ".into()]), None);
    }

    #[test]
    fn replacements_are_whole_word_case_insensitive_and_ordered() {
        let r = vec![
            ("b p".to_string(), "BP".to_string()),
            ("para".to_string(), "paracetamol".to_string()),
        ];
        assert_eq!(
            apply_replacements("The b p was high, gave para.", &r),
            "The BP was high, gave paracetamol."
        );
        assert_eq!(
            apply_replacements("B P at the start", &r),
            "BP at the start"
        );
        assert_eq!(
            apply_replacements("paragraph is not para", &r),
            "paragraph is not paracetamol"
        );
        assert_eq!(apply_replacements("", &r), "");
        assert_eq!(apply_replacements("no change", &[]), "no change");
        // Unicode around the match does not break the boundary check.
        assert_eq!(apply_replacements("é para é", &r), "é paracetamol é");
    }

    #[test]
    fn non_speech_markers_are_removed() {
        assert_eq!(clean_transcript(" [BLANK_AUDIO] "), "");
        assert_eq!(
            clean_transcript("Take two tablets (coughs) daily. [MUSIC] ♪"),
            "Take two tablets daily."
        );
        assert_eq!(clean_transcript("*laughs* fine *sighs*  then"), "fine then");
        assert_eq!(clean_transcript("BP 120 over 80"), "BP 120 over 80");
        // Unbalanced closers are kept as text, not swallowed.
        assert_eq!(clean_transcript("odd ) text"), "odd ) text");
    }
}
