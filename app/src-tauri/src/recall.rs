//! Recall: how a question is answered from the personal context store
//! (knowledge/25, "How a question is answered"). Rules in Rust turn the
//! question into typed lookups — a name becomes a person, a date expression
//! becomes a range, the rest becomes keywords — and the matching lookup runs.
//! A pure lookup ("the last thing Tunde said") is rendered directly; only a
//! question about *words* hands a few exact records to the model.
//!
//! Nothing here calls a model. Small models are misled by retrieved text far
//! more than they are helped by it (Pandey et al. 2026, arXiv 2603.11513), so
//! the fewer and more exact the records the better, and the model is the last
//! step, not the first.

use regex::Regex;

use crate::context::{self, Event, Observation, Person, PersonMatch, Store};
use crate::web::Snippet;

/// Wall-clock input for date expressions. `tz_offset_min` is minutes EAST of
/// UTC (Lagos = +60), the sign JavaScript's `getTimezoneOffset()` negates.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Clock {
    pub now_ms: i64,
    pub tz_offset_min: i32,
}

/// What recall found for one question.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Recall {
    /// `none` (not a question for the store), `direct` (answered without a
    /// model), `sources` (records for the model to answer from), `ambiguous`
    /// (a name matched several people — ask which).
    pub kind: &'static str,
    /// What was looked up, for the footer under the answer.
    pub label: String,
    pub direct: Option<String>,
    pub sources: Vec<Snippet>,
    pub ambiguous: Vec<Person>,
}

impl Recall {
    fn none() -> Recall {
        Recall { kind: "none", label: String::new(), direct: None, sources: Vec::new(), ambiguous: Vec::new() }
    }
}

/// How many records a question may hand the model (knowledge/25: 8–12 at most).
const MAX_RECORDS: usize = 8;

/// Words that say "this is about what my phone has seen", not general
/// knowledge. Whole words, lower case.
const CUES: &[&str] = &[
    "said", "say", "says", "told", "tell", "sent", "send", "wrote", "write", "texted", "messaged", "message",
    "messages", "chat", "chats", "whatsapp", "telegram", "instagram", "messenger", "sms", "conversation",
    "remind", "mentioned", "mention", "promised", "asked", "replied", "reply", "last", "latest", "recent",
    "recently", "event", "events", "meeting", "appointment", "calendar", "wedding", "birthday", "plans",
    "plan", "doing", "see", "saw", "seen", "met", "call", "called", "photo", "photos", "schedule",
    "happened", "happen", "happening", "going",
];

/// Words recall itself consumes; they are never keywords or names.
const FUNCTION_WORDS: &[&str] = &[
    "today", "tonight", "yesterday", "tomorrow", "this", "next", "week", "month", "year", "morning",
    "afternoon", "evening", "weekend", "days", "ago", "monday", "tuesday", "wednesday", "thursday", "friday",
    "saturday", "sunday", "january", "february", "march", "april", "may", "june", "july", "august",
    "september", "october", "november", "december", "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep",
    "sept", "oct", "nov", "dec", "thing", "things", "time", "did", "was", "were", "when", "what", "who",
    "whose", "where", "with", "about", "from", "the", "and", "for", "did", "does", "has", "have", "had",
    "anything", "something", "everything", "all", "any", "me", "my", "i'm", "im", "am",
];

/// Answer `question` from the store, by rules. `Recall::none` means the
/// question is not about what the phone has seen and the caller should go on
/// (to the web, or to the model alone).
pub(crate) fn recall(store: &Store, question: &str, clock: Clock) -> Result<Recall, String> {
    let lower = question.to_lowercase();
    let words: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .map(|w| w.trim_matches('\'').to_string())
        .filter(|w| !w.is_empty())
        .collect();
    let has = |w: &str| words.iter().any(|x| x == w);
    let cue = words.iter().any(|w| CUES.contains(&w.as_str()));
    let past = ["did", "said", "was", "were", "sent", "told", "wrote", "last", "yesterday", "ago"].iter().any(|w| has(w));

    // 1. The date expression, if any.
    let date = date_range(&lower, clock, past);

    // 2. The person, if any: a two-word name first, then single words.
    let mut person: Option<Person> = None;
    let mut used: Vec<String> = Vec::new();
    let candidates: Vec<(String, Vec<String>)> = words
        .windows(2)
        .filter(|p| p.iter().all(|w| name_candidate(w)))
        .map(|p| (format!("{} {}", p[0], p[1]), p.to_vec()))
        .chain(words.iter().filter(|w| name_candidate(w)).map(|w| (w.clone(), vec![w.clone()])))
        .collect();
    for (name, parts) in candidates {
        match store.find_person(&name)? {
            PersonMatch::One(p) => {
                person = Some(p);
                used = parts;
                break;
            }
            PersonMatch::Ambiguous(ps) => {
                return Ok(Recall {
                    kind: "ambiguous",
                    label: name,
                    direct: None,
                    sources: Vec::new(),
                    ambiguous: ps,
                })
            }
            PersonMatch::None => {}
        }
    }

    if !cue && person.is_none() && date.is_none() {
        return Ok(Recall::none());
    }

    // 3. What is left is the keywords.
    let keywords: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .filter(|w| w.len() >= 2 && !used.iter().any(|u| u == w) && !CUES.contains(w) && !FUNCTION_WORDS.contains(w) && !context::is_stopword(w))
        // The date expression's own numbers ("the 25th", "dec 25") are not keywords.
        .filter(|w| date.is_none() || !w.chars().any(|c| c.is_ascii_digit()))
        .collect();
    let topic = keywords.join(" ");

    // "When did I last see Ada" / "events with Ada": the participation link.
    if let Some(p) = &person {
        if (has("see") || has("saw") || has("seen") || has("met") || has("events") || has("event") || has("meeting"))
            && keywords.is_empty()
        {
            let events = store.events_with(p.id, 5)?;
            if !events.is_empty() {
                let label = format!("Events with {}", p.display_name);
                let direct = render_events(&format!("Events with {}:", p.display_name), &events);
                return Ok(Recall { kind: "direct", label, direct: Some(direct), sources: event_snippets(store, &events)?, ambiguous: Vec::new() });
            }
        }
    }

    match (&person, &date, keywords.is_empty()) {
        // "The last thing Tunde said": an indexed lookup, rendered directly.
        (Some(p), None, true) => {
            let items = store.last_from(p.id, 3)?;
            if items.is_empty() {
                return Ok(Recall::none());
            }
            let label = format!("The last thing {} said", p.display_name);
            let first = &items[0];
            let direct = format!(
                "The last thing {} said was on {}, {}: “{}”",
                p.display_name,
                human_date(first.at_ms),
                context::app_label(&first.app),
                first.text.trim()
            );
            Ok(Recall { kind: "direct", label, direct: Some(direct), sources: context::observations_as_snippets(&items), ambiguous: Vec::new() })
        }
        // "What did Ada send last week": a range, scoped to them.
        (Some(p), Some((start, end, when)), true) => {
            let items = store.in_range(*start, *end, Some(p.id), MAX_RECORDS)?;
            let label = format!("From {}, {when}", p.display_name);
            if items.is_empty() {
                let direct = format!("Nothing from {} {when}.", p.display_name);
                return Ok(Recall { kind: "direct", label, direct: Some(direct), sources: Vec::new(), ambiguous: Vec::new() });
            }
            let direct = render_items(&format!("From {}, {when}:", p.display_name), &items);
            Ok(Recall { kind: "direct", label, direct: Some(direct), sources: context::observations_as_snippets(&items), ambiguous: Vec::new() })
        }
        // "What am I doing on the 25th" / "what happened yesterday": the day.
        (None, Some((start, end, when)), true) => {
            let events = store.events_in_range(*start, *end, 5)?;
            let items = store.in_range(*start, *end, None, MAX_RECORDS)?;
            let label = capitalise(when);
            if events.is_empty() && items.is_empty() {
                let direct = format!("Nothing on your phone {when}.");
                return Ok(Recall { kind: "direct", label, direct: Some(direct), sources: Vec::new(), ambiguous: Vec::new() });
            }
            let mut direct = String::new();
            if !events.is_empty() {
                direct.push_str(&render_events(&format!("{}:", capitalise(when)), &events));
            }
            if !items.is_empty() {
                if !direct.is_empty() {
                    direct.push_str("\n\n");
                }
                direct.push_str(&render_items(&format!("Messages {when}:"), &items));
            }
            let mut sources = event_snippets(store, &events)?;
            sources.extend(context::observations_as_snippets(&items));
            Ok(Recall { kind: "direct", label, direct: Some(direct), sources, ambiguous: Vec::new() })
        }
        // A question about words: the keyword tier, scoped if a person was named.
        _ => {
            let scope = person.as_ref().map(|p| p.id);
            let mut items = store.search(&topic, scope, MAX_RECORDS)?;
            if let Some((start, end, _)) = &date {
                items.retain(|i| i.at_ms >= *start && i.at_ms < *end);
            }
            let events = if person.is_none() { store.events_titled(&topic, 3)? } else { Vec::new() };
            if items.is_empty() && events.is_empty() {
                // Named someone but their words didn't match: their latest
                // messages are the honest fallback, for the model to read.
                if let Some(p) = &person {
                    let latest = store.last_from(p.id, 5)?;
                    if !latest.is_empty() {
                        return Ok(Recall {
                            kind: "sources",
                            label: format!("Recent messages from {}", p.display_name),
                            direct: None,
                            sources: context::observations_as_snippets(&latest),
                            ambiguous: Vec::new(),
                        });
                    }
                }
                return Ok(Recall::none());
            }
            let label = match &person {
                Some(p) => format!("{}'s messages about {topic}", p.display_name),
                None => format!("Your messages about {topic}"),
            };
            let mut sources = event_snippets(store, &events)?;
            sources.extend(context::observations_as_snippets(&items));
            sources.truncate(MAX_RECORDS);
            Ok(Recall { kind: "sources", label, direct: None, sources, ambiguous: Vec::new() })
        }
    }
}

/// The heading the model sees above recalled records (knowledge/25). Private
/// by construction: the records are the user's own, already on the device.
pub(crate) fn sources_heading(label: &str) -> String {
    format!("From the user's own phone — {label} (their messages and calendar, already on this device):")
}

fn name_candidate(w: &str) -> bool {
    w.len() >= 3 && !CUES.contains(&w) && !FUNCTION_WORDS.contains(&w) && w.chars().all(|c| c.is_alphabetic() || c == '\'')
}

fn event_snippets(store: &Store, events: &[Event]) -> Result<Vec<Snippet>, String> {
    let mut with_people = Vec::with_capacity(events.len());
    for e in events {
        with_people.push((e.clone(), store.people_at(e.id)?));
    }
    Ok(context::events_as_snippets(&with_people))
}

fn render_items(heading: &str, items: &[Observation]) -> String {
    let mut s = heading.to_string();
    // Oldest first reads as a story; the store returns newest first.
    for i in items.iter().rev() {
        let who = match (i.kind, i.direction) {
            ("message", "out") => "You".to_string(),
            ("message", _) => i.sender.clone(),
            ("call", _) if !i.sender.is_empty() => format!("Call with {}", i.sender),
            (k, _) => capitalise(k),
        };
        let text = i.text.trim().replace('\n', " ");
        let text = if text.chars().count() > 160 { format!("{}…", text.chars().take(160).collect::<String>()) } else { text };
        s.push_str(&format!("\n- {} ({}): {}", who, human_date(i.at_ms), text));
    }
    s
}

fn render_events(heading: &str, events: &[Event]) -> String {
    let mut s = heading.to_string();
    for e in events {
        s.push_str(&format!("\n- {} — {}", e.title, human_date(e.start_ms)));
        if let Some(p) = &e.place {
            s.push_str(&format!(", at {p}"));
        }
    }
    s
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/// "12 May 2026", from unix milliseconds (UTC date, like the snippets).
pub(crate) fn human_date(at_ms: i64) -> String {
    let (y, m, d) = civil(at_ms.div_euclid(86_400_000));
    format!("{d} {} {y}", MONTHS[(m - 1) as usize])
}

/// Civil date from days since the epoch (Howard Hinnant's algorithm).
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Days since the epoch for a civil date.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    context::to_unix_ms(y, m, d, 0, 0, 0) / 86_400_000
}

/// 0 = Monday … 6 = Sunday, for a day index (1970-01-01 was a Thursday).
fn weekday(days: i64) -> i64 {
    (days + 3).rem_euclid(7)
}

const DAY_MS: i64 = 86_400_000;

/// A date expression in `lower` → `[start, end)` in unix ms plus a label, in
/// the device's local time. `past` biases ambiguous expressions ("the 25th",
/// "Dec 25", "Friday") towards the most recent occurrence instead of the next.
/// A rule, never the model's job (knowledge/25).
pub(crate) fn date_range(lower: &str, clock: Clock, past: bool) -> Option<(i64, i64, String)> {
    let off = clock.tz_offset_min as i64 * 60_000;
    let local_now = clock.now_ms + off;
    let today = local_now.div_euclid(DAY_MS);
    let (y, m, d) = civil(today);
    // Local day index → UTC range.
    let day = |idx: i64| (idx * DAY_MS - off, (idx + 1) * DAY_MS - off);
    let span = |a: i64, b: i64| (a * DAY_MS - off, b * DAY_MS - off);
    let month_span = |yy: i64, mm: i64| {
        let (ny, nm) = if mm == 12 { (yy + 1, 1) } else { (yy, mm + 1) };
        span(days_from_civil(yy, mm, 1), days_from_civil(ny, nm, 1))
    };
    let has = |w: &str| Regex::new(&format!(r"\b{w}\b")).unwrap().is_match(lower);

    if has("today") {
        let (s, e) = day(today);
        return Some((s, e, "today".into()));
    }
    if has("tonight") || has("this evening") {
        let (s, e) = day(today);
        return Some((s + 17 * 3_600_000, e, "this evening".into()));
    }
    if has("this morning") {
        let (s, _) = day(today);
        return Some((s, s + 12 * 3_600_000, "this morning".into()));
    }
    if has("this afternoon") {
        let (s, _) = day(today);
        return Some((s + 12 * 3_600_000, s + 18 * 3_600_000, "this afternoon".into()));
    }
    if has("yesterday") {
        let (s, e) = day(today - 1);
        return Some((s, e, "yesterday".into()));
    }
    if has("tomorrow") {
        let (s, e) = day(today + 1);
        return Some((s, e, "tomorrow".into()));
    }
    let monday = today - weekday(today);
    if has("this week") {
        let (s, e) = span(monday, monday + 7);
        return Some((s, e, "this week".into()));
    }
    if has("last week") {
        let (s, e) = span(monday - 7, monday);
        return Some((s, e, "last week".into()));
    }
    if has("next week") {
        let (s, e) = span(monday + 7, monday + 14);
        return Some((s, e, "next week".into()));
    }
    if has("this weekend") || has("the weekend") {
        let (s, e) = span(monday + 5, monday + 7);
        return Some((s, e, "this weekend".into()));
    }
    if has("this month") {
        let (s, e) = month_span(y, m);
        return Some((s, e, "this month".into()));
    }
    if has("last month") {
        let (py, pm) = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
        let (s, e) = month_span(py, pm);
        return Some((s, e, "last month".into()));
    }
    if has("next month") {
        let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        let (s, e) = month_span(ny, nm);
        return Some((s, e, "next month".into()));
    }
    if has("this year") {
        let (s, e) = span(days_from_civil(y, 1, 1), days_from_civil(y + 1, 1, 1));
        return Some((s, e, "this year".into()));
    }
    if has("last year") {
        let (s, e) = span(days_from_civil(y - 1, 1, 1), days_from_civil(y, 1, 1));
        return Some((s, e, "last year".into()));
    }
    if let Some(c) = Regex::new(r"\b(?:last|past) (\d{1,3}) days\b").unwrap().captures(lower) {
        let n: i64 = c[1].parse().unwrap_or(7);
        let (s, e) = span(today - n, today + 1);
        return Some((s, e, format!("in the last {n} days")));
    }
    if let Some(c) = Regex::new(r"\b(\d{1,3}) days ago\b").unwrap().captures(lower) {
        let n: i64 = c[1].parse().unwrap_or(1);
        let (s, e) = day(today - n);
        return Some((s, e, format!("{n} days ago")));
    }
    // A weekday: "last friday" / "on friday" / "next friday".
    const DAYS: [&str; 7] = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
    if let Some(c) = Regex::new(r"\b(last |next |on |this )?(monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b").unwrap().captures(lower) {
        let target = DAYS.iter().position(|dn| *dn == &c[2]).unwrap() as i64;
        let qualifier = c.get(1).map(|q| q.as_str().trim()).unwrap_or("");
        let delta = target - weekday(today);
        let idx = match qualifier {
            "next" => today + if delta > 0 { delta } else { delta + 7 },
            "last" => today + if delta < 0 { delta } else { delta - 7 },
            _ if past => today + if delta <= 0 { delta } else { delta - 7 },
            _ => today + if delta >= 0 { delta } else { delta + 7 },
        };
        let (s, e) = day(idx);
        return Some((s, e, format!("on {}", capitalise(&c[2]))));
    }
    // A month and day: "dec 25", "25 dec", "december 25th", "25th of december".
    let month_re = r"(jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t|tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)";
    let md = Regex::new(&format!(r"\b{month_re}\.? (\d{{1,2}})(?:st|nd|rd|th)?\b")).unwrap();
    let dm = Regex::new(&format!(r"\b(\d{{1,2}})(?:st|nd|rd|th)? (?:of )?{month_re}\b")).unwrap();
    let parsed = md
        .captures(lower)
        .map(|c| (month_index(&c[1]), c[2].parse::<i64>().unwrap_or(1)))
        .or_else(|| dm.captures(lower).map(|c| (month_index(&c[2]), c[1].parse::<i64>().unwrap_or(1))));
    if let Some((mm, dd)) = parsed {
        if (1..=31).contains(&dd) {
            let mut yy = y;
            let this_year = days_from_civil(yy, mm, dd);
            if past && this_year > today {
                yy -= 1;
            } else if !past && this_year < today {
                yy += 1;
            }
            let idx = days_from_civil(yy, mm, dd);
            let (s, e) = day(idx);
            return Some((s, e, format!("on {dd} {}", MONTHS[(mm - 1) as usize])));
        }
    }
    // A bare day of the month: "on the 25th".
    if let Some(c) = Regex::new(r"\bthe (\d{1,2})(?:st|nd|rd|th)\b").unwrap().captures(lower) {
        let dd: i64 = c[1].parse().unwrap_or(1);
        if (1..=31).contains(&dd) {
            let (mut yy, mut mm) = (y, m);
            if past && dd > d {
                if mm == 1 {
                    yy -= 1;
                    mm = 12;
                } else {
                    mm -= 1;
                }
            } else if !past && dd < d {
                if mm == 12 {
                    yy += 1;
                    mm = 1;
                } else {
                    mm += 1;
                }
            }
            let (s, e) = day(days_from_civil(yy, mm, dd));
            return Some((s, e, format!("on the {dd}{}", ordinal(dd))));
        }
    }
    // A bare month: "in december". "may" needs a qualifier ("in may"), or every
    // "you may be right" would become a date.
    if let Some(c) = Regex::new(&format!(r"\b((?:in|last|next|this) )?{month_re}\b")).unwrap().captures(lower) {
        if &c[2] == "may" && c.get(1).is_none() {
            return None;
        }
        let mm = month_index(&c[2]);
        let mut yy = y;
        if past && mm > m {
            yy -= 1;
        } else if !past && mm < m {
            yy += 1;
        }
        let (s, e) = month_span(yy, mm);
        return Some((s, e, format!("in {}", MONTHS[(mm - 1) as usize])));
    }
    // A year.
    if let Some(c) = Regex::new(r"\b(20\d\d)\b").unwrap().captures(lower) {
        let yy: i64 = c[1].parse().unwrap_or(y);
        let (s, e) = span(days_from_civil(yy, 1, 1), days_from_civil(yy + 1, 1, 1));
        return Some((s, e, format!("in {yy}")));
    }
    None
}

fn month_index(name: &str) -> i64 {
    match &name[..3] {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        _ => 12,
    }
}

fn ordinal(d: i64) -> &'static str {
    match d % 100 {
        11..=13 => "th",
        _ => match d % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{source, Direction, NewEvent, NewMessage, Origin, Role};

    // Thursday 22 October 2026, 10:00 UTC; the device is in Lagos (UTC+1).
    const NOW: i64 = 1_792_670_400_000;
    const CLOCK: Clock = Clock { now_ms: NOW, tz_offset_min: 60 };

    fn ms(y: i64, m: i64, d: i64, h: i64) -> i64 {
        context::to_unix_ms(y, m, d, h, 0, 0)
    }

    fn msg(app: &str, thread: &str, sender: &str, at_ms: i64, text: &str) -> NewMessage {
        NewMessage {
            source: source::NOTIFICATION.to_string(),
            app: app.to_string(),
            thread_key: thread.to_lowercase(),
            thread_title: thread.to_string(),
            is_group: false,
            sender: sender.to_string(),
            sender_key: sender.to_string(),
            direction: if sender.is_empty() { Direction::Out } else { Direction::In },
            at_ms,
            text: text.to_string(),
        }
    }

    fn seeded() -> Store {
        let mut s = Store::open_in_memory().unwrap();
        s.ingest(&msg("com.whatsapp", "Tunde", "Tunde", ms(2026, 10, 12, 9), "Rent is due on the 5th, landlord wants 1.2m")).unwrap();
        s.ingest(&msg("com.whatsapp", "Tunde", "", ms(2026, 10, 12, 10), "Ok I'll send my half on Friday")).unwrap();
        s.ingest(&msg("org.telegram.messenger", "Ada", "Ada", ms(2026, 10, 15, 14), "Happy to help with the visa forms")).unwrap();
        s.ingest(&msg("com.whatsapp", "Tunde", "Tunde", ms(2026, 10, 21, 20), "See you Friday then")).unwrap();
        s.ingest(&msg("com.whatsapp", "Ada", "Ada", ms(2026, 10, 22, 8), "Morning! Bring the forms")).unwrap();
        let ada = match s.find_person("Ada").unwrap() {
            PersonMatch::One(p) => p,
            _ => panic!(),
        };
        let ev = s
            .upsert_event(&NewEvent {
                title: "Lunch with Ada".into(),
                start_ms: ms(2026, 10, 25, 12),
                end_ms: ms(2026, 10, 25, 14),
                place: Some("The Wheatbaker".into()),
                origin: Origin::Given,
                app: "com.google.android.calendar".into(),
                source_key: "ev-1".into(),
            })
            .unwrap();
        s.add_participant(ev, ada.id, Role::Invited, source::CALENDAR).unwrap();
        s
    }

    #[test]
    fn a_general_knowledge_question_is_not_for_the_store() {
        let s = seeded();
        let r = recall(&s, "What's the chemical composition of water?", CLOCK).unwrap();
        assert_eq!(r.kind, "none");
        // A cue word alone, with nothing stored about it, is still none.
        assert_eq!(recall(&s, "what did they say about quantum physics", CLOCK).unwrap().kind, "none");
    }

    #[test]
    fn the_last_thing_someone_said_is_answered_without_a_model() {
        let s = seeded();
        let r = recall(&s, "what was the last thing tunde said?", CLOCK).unwrap();
        assert_eq!(r.kind, "direct");
        assert_eq!(r.direct.as_deref(), Some("The last thing Tunde said was on 21 October 2026, WhatsApp: “See you Friday then”"));
        assert_eq!(r.sources.len(), 2, "both of his messages ride along as sources");
        assert_eq!(r.label, "The last thing Tunde said");
    }

    #[test]
    fn a_person_and_a_date_is_a_scoped_range() {
        let s = seeded();
        let r = recall(&s, "what did Ada send today?", CLOCK).unwrap();
        assert_eq!(r.kind, "direct");
        assert!(r.direct.as_deref().unwrap().contains("Bring the forms"));
        assert!(!r.direct.as_deref().unwrap().contains("visa"), "last week's message is outside today");
        let none = recall(&s, "what did Ada send last year", CLOCK).unwrap();
        assert_eq!(none.direct.as_deref(), Some("Nothing from Ada last year."));
    }

    #[test]
    fn a_day_alone_lists_events_and_messages() {
        let s = seeded();
        let r = recall(&s, "what am I doing on the 25th?", CLOCK).unwrap();
        assert_eq!(r.kind, "direct");
        let d = r.direct.unwrap();
        assert!(d.starts_with("On the 25th:\n- Lunch with Ada — 25 October 2026, at The Wheatbaker"), "{d}");
        assert_eq!(r.sources[0].domain, "Calendar");
        let y = recall(&s, "what happened yesterday", CLOCK).unwrap();
        assert!(y.direct.unwrap().contains("See you Friday then"));
    }

    #[test]
    fn events_with_a_person_use_the_participation_link() {
        let s = seeded();
        let r = recall(&s, "when did I last see Ada?", CLOCK).unwrap();
        assert_eq!(r.kind, "direct");
        assert!(r.direct.unwrap().starts_with("Events with Ada:\n- Lunch with Ada"));
    }

    #[test]
    fn a_question_about_words_hands_records_to_the_model() {
        let s = seeded();
        let r = recall(&s, "what did tunde say about the rent?", CLOCK).unwrap();
        assert_eq!(r.kind, "sources");
        assert_eq!(r.label, "Tunde's messages about rent");
        assert!(r.sources[0].text.starts_with("Rent is due"));
        assert!(r.sources.iter().all(|x| x.title.starts_with("Tunde")), "scoped to him");
        // Unscoped: the words alone.
        let v = recall(&s, "who mentioned the visa forms?", CLOCK).unwrap();
        assert_eq!(v.kind, "sources");
        assert_eq!(v.sources[0].domain, "Telegram");
        // Named someone whose words don't match: their latest, for the model.
        let f = recall(&s, "did tunde mention the generator?", CLOCK).unwrap();
        assert_eq!(f.kind, "sources");
        assert_eq!(f.label, "Recent messages from Tunde");
    }

    #[test]
    fn two_people_with_one_name_become_a_question_back() {
        let mut s = seeded();
        s.ingest(&msg("com.whatsapp", "Jackson Lee", "Jackson Lee", ms(2026, 10, 1, 1), "hey")).unwrap();
        s.ingest(&msg("com.whatsapp", "Jackson Okoro", "Jackson Okoro", ms(2026, 10, 2, 1), "yo")).unwrap();
        let r = recall(&s, "what did Jackson say?", CLOCK).unwrap();
        assert_eq!(r.kind, "ambiguous");
        assert_eq!(r.ambiguous.len(), 2);
        assert_eq!(r.label, "jackson");
    }

    #[test]
    fn date_expressions_resolve_in_local_time() {
        let day = |y, m, d| (ms(y, m, d, 0) - 3_600_000, ms(y, m, d + 1, 0) - 3_600_000);
        let (s, e, l) = date_range("today", CLOCK, false).unwrap();
        assert_eq!((s, e), day(2026, 10, 22));
        assert_eq!(l, "today");
        assert_eq!(date_range("yesterday", CLOCK, true).unwrap().0, day(2026, 10, 21).0);
        // Thursday 22 Oct: this week is Mon 19 – Sun 25; last week Mon 12 – Sun 18.
        let (s, e, _) = date_range("this week", CLOCK, false).unwrap();
        assert_eq!((s, e), (day(2026, 10, 19).0, day(2026, 10, 26).0));
        let (s, _, _) = date_range("last week", CLOCK, true).unwrap();
        assert_eq!(s, day(2026, 10, 12).0);
        // "Friday" in the past tense is last Friday (16th); otherwise this Friday (23rd).
        assert_eq!(date_range("what did he say on friday", CLOCK, true).unwrap().0, day(2026, 10, 16).0);
        assert_eq!(date_range("what's on friday", CLOCK, false).unwrap().0, day(2026, 10, 23).0);
        assert_eq!(date_range("last thursday", CLOCK, true).unwrap().0, day(2026, 10, 15).0);
        // Month + day, both orders, with the year chosen by tense.
        assert_eq!(date_range("dec 25", CLOCK, false).unwrap().0, day(2026, 12, 25).0);
        assert_eq!(date_range("25th of december", CLOCK, false).unwrap().2, "on 25 December");
        assert_eq!(date_range("what happened on march 3rd", CLOCK, true).unwrap().0, day(2026, 3, 3).0);
        assert_eq!(date_range("what did she say on dec 25", CLOCK, true).unwrap().0, day(2025, 12, 25).0);
        // A bare day of the month: upcoming unless past tense.
        assert_eq!(date_range("on the 25th", CLOCK, false).unwrap().0, day(2026, 10, 25).0);
        assert_eq!(date_range("on the 5th", CLOCK, false).unwrap().0, day(2026, 11, 5).0);
        assert_eq!(date_range("what did he send on the 5th", CLOCK, true).unwrap().0, day(2026, 10, 5).0);
        // Month, year, nothing.
        assert_eq!(date_range("in september", CLOCK, true).unwrap().2, "in September");
        assert_eq!(date_range("back in 2025", CLOCK, true).unwrap().0, day(2025, 1, 1).0);
        assert!(date_range("the rent", CLOCK, false).is_none());
        // "may" the modal verb is not May the month; "in may" is.
        assert!(date_range("you may be right", CLOCK, false).is_none());
        assert_eq!(date_range("what did she say in may", CLOCK, true).unwrap().2, "in May");
    }

    #[test]
    fn human_dates_and_weekdays() {
        assert_eq!(human_date(ms(2026, 10, 22, 10)), "22 October 2026");
        assert_eq!(weekday(ms(2026, 10, 22, 0) / DAY_MS), 3, "a Thursday");
        assert_eq!(weekday(0), 3, "1970-01-01 was a Thursday");
        assert_eq!(ordinal(1), "st");
        assert_eq!(ordinal(11), "th");
        assert_eq!(ordinal(22), "nd");
        assert_eq!(sources_heading("Your messages about rent"), "From the user's own phone — Your messages about rent (their messages and calendar, already on this device):");
    }
}
