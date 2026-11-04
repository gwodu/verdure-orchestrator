//! `extract(text, schema)` — the product (knowledge/26, Part 3 §2): free text
//! in, JSON that matches the vendor's schema out, or an error that says why.
//!
//! The pipeline: write the schema into the prompt (llama.cpp never injects a
//! grammar's schema itself), generate, find the JSON in the reply, drop the
//! nulls the grammar convention allows, validate, and on a miss give the
//! model the violations once and try again. A grammar-constrained sampler is
//! available behind `ExtractOptions::use_grammar` but **off by default**: the
//! app removed llama.cpp's grammar sampler after it `ggml_abort()`ed the whole
//! process on a multi-byte token (knowledge/18, 2026-06-23), and an abort
//! takes the vendor's app down with it. It returns when the engine runs in an
//! isolated process or the path is re-verified — `[OQ-108]`.

use serde_json::Value;

use crate::engine::{Control, GenRequest, LlmEngine, Msg};
use crate::event::{Event, Progress};
use crate::VError;

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractOptions {
    /// Constrain sampling with a GBNF grammar rendered from the schema.
    pub use_grammar: bool,
    pub max_tokens: u32,
    /// How many times the model is shown its violations and asked again.
    pub repair_attempts: u32,
    /// Optional domain instruction prepended to the system prompt
    /// ("You are reading a community health worker's visit summary…").
    pub context: Option<String>,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        ExtractOptions {
            use_grammar: false,
            max_tokens: 1024,
            repair_attempts: 1,
            context: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extraction {
    /// Compact JSON, valid against the schema.
    pub json: String,
    /// Generations it took (1 = first try).
    pub attempts: u32,
    pub used_grammar: bool,
    /// From the grammar renderer: keywords enforced only by validation.
    pub warnings: Vec<String>,
}

/// The rules the model is given. Short on purpose — these models follow a
/// short rule better than a careful one (the app's lesson, knowledge/21), and
/// the two that matter most are the release gates: never invent, especially
/// numbers (CHW: "zero fabricated numeric vitals").
const SYSTEM: &str = "You turn text into JSON by filling in an outline. \
Use only what the text says. If the text does not state a value, use null. \
Never invent numbers, names or dates. \
Reply with the JSON only.\n\n\
Example.\n\
Text: Mrs Okafor is 34, her blood pressure is one fifty over ninety five, no known allergies.\n\
Outline: {\"name\": \"<text>\", \"age\": \"<integer or null>\", \"bp\": \"<text or null>\", \"allergies\": [\"<text>\"], \"smoker\": \"<true or false or null>\"}\n\
JSON: {\"name\": \"Mrs Okafor\", \"age\": 34, \"bp\": \"150/95\", \"allergies\": [], \"smoker\": null}";

pub fn extract(
    engine: &dyn LlmEngine,
    text: &str,
    schema_json: &str,
    opts: &ExtractOptions,
    progress: &dyn Progress,
) -> Result<Extraction, VError> {
    let schema: Value =
        serde_json::from_str(schema_json).map_err(|e| VError::InvalidSchema(e.to_string()))?;
    if !schema.is_object() {
        return Err(VError::InvalidSchema(
            "the schema must be a JSON object".into(),
        ));
    }
    let (grammar, warnings) = if opts.use_grammar {
        let g =
            verdure_grammar::to_gbnf(&schema).map_err(|e| VError::InvalidSchema(e.to_string()))?;
        (Some(g.text), g.warnings)
    } else {
        // Render anyway so a vendor hears about unsupported keywords early,
        // whether or not they switch the grammar on.
        let warnings = verdure_grammar::to_gbnf(&schema)
            .map(|g| g.warnings)
            .unwrap_or_default();
        (None, warnings)
    };

    let system = match &opts.context {
        Some(c) => format!("{c}\n\n{SYSTEM}"),
        None => SYSTEM.to_string(),
    };
    // The model sees an OUTLINE with placeholders, never the schema itself: on
    // the first real run a 0.5B model copied the schema back verbatim until its
    // budget ran out (`verdure_grammar::shape`). Validation uses the real schema.
    let outline = verdure_grammar::shape(&schema);
    let hints = description_hints(&schema);
    let mut messages = vec![
        Msg::system(system),
        Msg::user(format!("Text: {text}\nOutline: {outline}\nJSON:")),
    ];

    let total_attempts = 1 + opts.repair_attempts;
    for attempt in 1..=total_attempts {
        if progress.is_cancelled() {
            return Err(VError::Cancelled);
        }
        progress.on_event(Event::Progress {
            done: (attempt - 1) as u64,
            total: total_attempts as u64,
        });
        let req = GenRequest {
            messages: messages.clone(),
            max_tokens: opts.max_tokens,
            temperature: 0.0,
            grammar: grammar.clone(),
        };
        let mut stop = false;
        let reply = engine.generate(&req, &mut |_| {
            if progress.is_cancelled() {
                stop = true;
                Control::Stop
            } else {
                Control::Continue
            }
        })?;
        if stop {
            return Err(VError::Cancelled);
        }

        let found = find_json(&reply);
        let violations: Vec<String> =
            match found.and_then(|s| serde_json::from_str::<Value>(s).ok()) {
                Some(value) => {
                    let value = verdure_grammar::strip_optional_nulls(&schema, value);
                    let mut v = verdure_grammar::validate(&schema, &value);
                    // A small model may hand the outline back with its
                    // placeholders still in: valid JSON, not an extraction.
                    v.extend(placeholders_left(&value, "$", &hints));
                    if v.is_empty() {
                        progress.on_event(Event::Progress {
                            done: total_attempts as u64,
                            total: total_attempts as u64,
                        });
                        return Ok(Extraction {
                            json: serde_json::to_string(&value).expect("value serialises"),
                            attempts: attempt,
                            used_grammar: grammar.is_some(),
                            warnings,
                        });
                    }
                    v.iter().map(ToString::to_string).collect()
                }
                None => vec![format!(
                    "$: {} (it began: {:?})",
                    if found.is_some() {
                        "the reply's JSON could not be parsed"
                    } else {
                        "the reply did not contain a JSON object"
                    },
                    reply.chars().take(160).collect::<String>()
                )],
            };
        if attempt == total_attempts {
            return Err(VError::SchemaInvalid { violations });
        }
        messages.push(Msg::assistant(reply));
        messages.push(Msg::user(format!(
            "That JSON has problems:\n- {}\nFor each one, put the value the text gives, or null when the text does not say. Return the corrected JSON only.",
            violations.join("\n- ")
        )));
    }
    unreachable!("the loop returns on its last attempt")
}

/// Strings still shaped like the outline's placeholders (`<text>`,
/// `<one of: a | b>`), as violations the repair turn can name.
fn placeholders_left(
    value: &Value,
    path: &str,
    hints: &[String],
) -> Vec<verdure_grammar::Violation> {
    let mut out = Vec::new();
    match value {
        // `<text>` with its brackets, or a schema description copied out
        // word for word (seen on PriMock57 consultation 02): neither is a value.
        Value::String(s)
            if (s.starts_with('<') && s.ends_with('>')) || hints.iter().any(|h| h == s.trim()) =>
        {
            out.push(verdure_grammar::Violation {
                path: path.to_string(),
                // What went wrong, not what to do about it: this sentence
                // reaches the developer's log through `SchemaInvalid`. The
                // instruction belongs to the repair turn below.
                message: format!("still holds the placeholder {s:?}"),
            })
        }
        Value::Object(map) => {
            for (k, v) in map {
                out.extend(placeholders_left(v, &format!("{path}.{k}"), hints));
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                out.extend(placeholders_left(v, &format!("{path}[{i}]"), hints));
            }
        }
        _ => {}
    }
    out
}

/// Every `description` string in the schema — the words the outline shows as
/// placeholder hints, which a small model may copy back as if they were values.
fn description_hints(schema: &Value) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                if let Some(Value::String(d)) = m.get("description") {
                    out.push(d.trim().to_string());
                }
                m.values().for_each(|x| walk(x, out));
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    walk(schema, &mut out);
    out
}

/// The first JSON object or array in `reply`, tolerant of prose and code
/// fences around it. Brackets inside strings are skipped.
pub fn find_json(reply: &str) -> Option<&str> {
    let bytes = reply.as_bytes();
    let start = reply.find(['{', '['])?;
    let open = bytes[start];
    let close = if open == b'{' { b'}' } else { b']' };
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return if b == close {
                        Some(&reply[start..=i])
                    } else {
                        None
                    };
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::FakeLlm;
    use crate::event::{NoProgress, Recorder};

    const SCHEMA: &str = r#"{
        "type": "object",
        "properties": {
            "chief_complaint": {"type": "string"},
            "heart_rate": {"type": "integer", "minimum": 20, "maximum": 250},
            "medications": {"type": "array", "items": {"type": "string"}}
        },
        "required": ["chief_complaint", "medications"],
        "additionalProperties": false
    }"#;

    #[test]
    fn a_good_first_reply_is_returned_compact_and_validated() {
        let llm = FakeLlm::scripted([
            "Here you go:\n```json\n{\"chief_complaint\": \"cough\", \"heart_rate\": null, \"medications\": [\"amoxicillin\"]}\n```",
        ]);
        let out = extract(
            &llm,
            "Patient has a cough, on amoxicillin.",
            SCHEMA,
            &ExtractOptions::default(),
            &NoProgress,
        )
        .unwrap();
        assert_eq!(
            out.json,
            r#"{"chief_complaint":"cough","medications":["amoxicillin"]}"#
        );
        assert_eq!(out.attempts, 1);
        assert!(!out.used_grammar);
        assert_eq!(
            out.warnings.len(),
            2,
            "minimum/maximum are validator-only: {:?}",
            out.warnings
        );
        // The schema and the text both reached the model, and the rule against inventing.
        let reqs = llm.requests.lock().unwrap();
        assert_eq!(reqs.len(), 1);
        assert!(reqs[0].messages[0].content.contains("Never invent numbers"));
        let prompt = &reqs[0].messages[1].content;
        assert!(
            prompt.contains("\"chief_complaint\": \"<text>\""),
            "{prompt}"
        );
        assert!(prompt.contains("\"heart_rate\": \"<integer>\""), "{prompt}");
        assert!(
            !prompt.contains("minimum"),
            "schema keywords must not reach the model"
        );
        assert!(reqs[0].messages[1].content.contains("Patient has a cough"));
        assert!(
            reqs[0].grammar.is_none(),
            "the grammar sampler is off by default"
        );
    }

    #[test]
    fn violations_are_fed_back_once_and_the_repair_is_accepted() {
        let llm = FakeLlm::scripted([
            r#"{"chief_complaint": "cough", "heart_rate": 300, "medications": "amoxicillin", "age": 4}"#,
            r#"{"chief_complaint": "cough", "heart_rate": 88, "medications": ["amoxicillin"]}"#,
        ]);
        let rec = Recorder::default();
        let out = extract(&llm, "…", SCHEMA, &ExtractOptions::default(), &rec).unwrap();
        assert_eq!(out.attempts, 2);
        assert_eq!(
            out.json,
            r#"{"chief_complaint":"cough","heart_rate":88,"medications":["amoxicillin"]}"#
        );
        let reqs = llm.requests.lock().unwrap();
        let repair = &reqs[1].messages;
        assert_eq!(
            repair.len(),
            4,
            "system, user, the bad reply, the correction request"
        );
        let ask = &repair[3].content;
        assert!(ask.contains("$.heart_rate: must be at most 250"), "{ask}");
        assert!(
            ask.contains("$.medications: must be array, got string"),
            "{ask}"
        );
        assert!(ask.contains("$.age: is not a declared property"), "{ask}");
        let events = rec.events.lock().unwrap();
        assert!(events.contains(&Event::Progress { done: 2, total: 2 }));
    }

    #[test]
    fn a_second_miss_is_an_error_that_names_the_problems() {
        let llm = FakeLlm::scripted(["not json at all", "{\"medications\": []}"]);
        let err = extract(&llm, "…", SCHEMA, &ExtractOptions::default(), &NoProgress).unwrap_err();
        assert_eq!(
            err,
            VError::SchemaInvalid {
                violations: vec!["$: missing required property \"chief_complaint\"".into()]
            }
        );
        assert!(err.to_string().contains("did not match the schema"));
        // When the model never produced JSON at all, the error shows what it
        // said instead — the first thing anyone debugging a bundle needs.
        let llm = FakeLlm::scripted(["I cannot do that.", "Sorry, still no."]);
        let err = extract(&llm, "…", SCHEMA, &ExtractOptions::default(), &NoProgress).unwrap_err();
        assert!(
            err.to_string().contains("it began: \"Sorry, still no.\""),
            "{err}"
        );
        let reqs = llm.requests.lock().unwrap();
        assert!(reqs[1].messages[3]
            .content
            .contains("it began: \"I cannot do that.\""));
    }

    #[test]
    fn an_outline_handed_back_with_placeholders_is_not_an_extraction() {
        let llm = FakeLlm::scripted([
            r#"{"chief_complaint": "<text>", "heart_rate": null, "medications": ["<text>"]}"#,
            r#"{"chief_complaint": "cough", "medications": []}"#,
        ]);
        let out = extract(
            &llm,
            "cough",
            SCHEMA,
            &ExtractOptions::default(),
            &NoProgress,
        )
        .unwrap();
        assert_eq!(out.attempts, 2);
        let reqs = llm.requests.lock().unwrap();
        let ask = &reqs[1].messages[3].content;
        assert!(
            ask.contains("$.chief_complaint: still holds the placeholder \"<text>\""),
            "{ask}"
        );
        assert!(ask.contains("$.medications[0]: still holds"), "{ask}");
        // A description copied back without its brackets is caught too.
        let schema = r#"{"type":"object","properties":{"a":{"type":"string","description":"one statement"}},"required":["a"],"additionalProperties":false}"#;
        let llm = FakeLlm::scripted([r#"{"a": "one statement"}"#, r#"{"a": "cough"}"#]);
        let out = extract(
            &llm,
            "cough",
            schema,
            &ExtractOptions::default(),
            &NoProgress,
        )
        .unwrap();
        assert_eq!(out.attempts, 2);
        assert_eq!(out.json, r#"{"a":"cough"}"#);
        // Unparseable JSON is reported as such, not as "no JSON".
        let llm = FakeLlm::scripted(["{\"a\": [1, ...]}", "{\"a\": [1, ...]}"]);
        let err = extract(&llm, "x", SCHEMA, &ExtractOptions::default(), &NoProgress).unwrap_err();
        assert!(err.to_string().contains("could not be parsed"), "{err}");
    }

    #[test]
    fn grammar_mode_passes_the_grammar_to_the_engine() {
        let llm = FakeLlm::scripted([
            r#"{"chief_complaint": "x", "heart_rate": null, "medications": []}"#,
        ]);
        let opts = ExtractOptions {
            use_grammar: true,
            ..ExtractOptions::default()
        };
        let out = extract(&llm, "…", SCHEMA, &opts, &NoProgress).unwrap();
        assert!(out.used_grammar);
        let reqs = llm.requests.lock().unwrap();
        let g = reqs[0].grammar.as_deref().unwrap();
        assert!(g.contains("root ::="), "{g}");
        assert!(
            g.contains(
                "root-heart-rate-kv ::= \"\\\"heart_rate\\\"\" space \":\" space (integer | null)"
            ),
            "{g}"
        );
    }

    #[test]
    fn bad_schemas_and_cancellation_are_reported_not_guessed_at() {
        let llm = FakeLlm::scripted(["{}"]);
        assert!(matches!(
            extract(
                &llm,
                "x",
                "not json",
                &ExtractOptions::default(),
                &NoProgress
            ),
            Err(VError::InvalidSchema(_))
        ));
        assert!(matches!(
            extract(&llm, "x", "[1]", &ExtractOptions::default(), &NoProgress),
            Err(VError::InvalidSchema(_))
        ));
        assert!(matches!(
            extract(&llm, "x", r#"{"type":"object","properties":{"a":{"type":"string"}}}"#, &ExtractOptions { use_grammar: true, ..Default::default() }, &NoProgress).map(|o| o.warnings),
            Ok(w) if w[0].contains("additionalProperties")
        ));
        let cancelled = Recorder {
            cancel_after: Some(0),
            ..Default::default()
        };
        assert_eq!(
            extract(&llm, "x", SCHEMA, &ExtractOptions::default(), &cancelled),
            Err(VError::Cancelled)
        );
    }

    #[test]
    fn find_json_skips_prose_fences_and_brackets_inside_strings() {
        assert_eq!(
            find_json("Sure! {\"a\": \"}\"} done"),
            Some("{\"a\": \"}\"}")
        );
        assert_eq!(find_json("```json\n[1, [2, 3]]\n```"), Some("[1, [2, 3]]"));
        assert_eq!(
            find_json("{\"a\": \"esc\\\"}\"}"),
            Some("{\"a\": \"esc\\\"}\"}")
        );
        assert_eq!(find_json("nothing here"), None);
        assert_eq!(find_json("{\"unterminated\": 1"), None);
        assert_eq!(find_json("{]"), None, "mismatched closer is not JSON");
    }
}
