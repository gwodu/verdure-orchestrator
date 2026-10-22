//! The JSON Schema → GBNF renderer. See the crate docs for the subset and the
//! one deviation (all properties emitted, optional ones nullable).

use std::collections::BTreeMap;

use serde_json::Value;

/// A rendered grammar. `text` is what `llama.cpp` parses; `warnings` lists
/// every schema keyword that was understood but could not be expressed in the
/// grammar (`pattern`, `format`, `minimum`…) — the validator still enforces
/// those, so they are not lost, only unconstrained at sampling time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grammar {
    pub text: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GrammarError {
    #[error("schema must be a JSON object, got {0}")]
    NotAnObject(String),
    #[error("unsupported schema at {path}: {reason}")]
    Unsupported { path: String, reason: String },
    #[error("unresolvable $ref {0}")]
    BadRef(String),
    #[error("$ref cycle through {0} (recursive schemas are not supported)")]
    RefCycle(String),
}

const PRIMITIVES: &[(&str, &str)] = &[
    ("space", r#"| " " | "\n" [ \t]{0,20}"#),
    (
        "string",
        r#""\"" ( [^"\\\x7F\x00-\x1F] | "\\" (["\\bfnrt] | "u" [0-9a-fA-F]{4}) )* "\"" space"#,
    ),
    (
        "number",
        r#"("-"? ([0-9] | [1-9] [0-9]{0,15})) ("." [0-9]+)? ([eE] [-+]? [0-9] [1-9]{0,15})? space"#,
    ),
    ("integer", r#"("-"? ([0-9] | [1-9] [0-9]{0,15})) space"#),
    ("boolean", r#"("true" | "false") space"#),
    ("null", r#""null" space"#),
];

struct Renderer<'s> {
    root: &'s Value,
    rules: BTreeMap<String, String>,
    order: Vec<String>,
    warnings: Vec<String>,
    /// `$ref` targets currently being rendered, to refuse recursion loudly.
    in_progress: Vec<String>,
}

/// Render `schema` as a GBNF grammar whose `root` rule accepts exactly the JSON
/// documents the schema accepts (within the subset).
pub fn to_gbnf(schema: &Value) -> Result<Grammar, GrammarError> {
    if !schema.is_object() {
        return Err(GrammarError::NotAnObject(short(schema)));
    }
    let mut r = Renderer {
        root: schema,
        rules: BTreeMap::new(),
        order: Vec::new(),
        warnings: Vec::new(),
        in_progress: Vec::new(),
    };
    let body = r.render(schema, "root", "$")?;
    r.add_rule("root", body);
    let mut text = String::new();
    for name in &r.order {
        text.push_str(name);
        text.push_str(" ::= ");
        text.push_str(&r.rules[name]);
        text.push('\n');
    }
    Ok(Grammar {
        text,
        warnings: r.warnings,
    })
}

impl<'s> Renderer<'s> {
    fn add_rule(&mut self, name: &str, body: String) -> String {
        if !self.rules.contains_key(name) {
            self.order.push(name.to_string());
        }
        self.rules.insert(name.to_string(), body);
        name.to_string()
    }

    /// Reference a primitive rule, making sure it is emitted.
    fn primitive(&mut self, name: &str) -> String {
        if !self.rules.contains_key(name) {
            let body = PRIMITIVES
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, b)| *b)
                .unwrap();
            self.add_rule(name, body.to_string());
            if name != "space" {
                self.primitive("space");
            }
        }
        name.to_string()
    }

    fn warn(&mut self, path: &str, what: &str) {
        self.warnings.push(format!(
            "{path}: `{what}` is enforced by the validator, not the grammar"
        ));
    }

    /// Render a schema node to a grammar *expression* (not a rule), creating
    /// named rules for anything compound. `name` is the preferred rule name
    /// for compound nodes; `path` is the JSON-schema path for messages.
    fn render(&mut self, schema: &Value, name: &str, path: &str) -> Result<String, GrammarError> {
        let obj = schema
            .as_object()
            .ok_or_else(|| GrammarError::Unsupported {
                path: path.to_string(),
                reason: format!("expected a schema object, got {}", short(schema)),
            })?;

        if let Some(Value::String(r)) = obj.get("$ref") {
            return self.render_ref(r, path);
        }
        if let Some(c) = obj.get("const") {
            self.primitive("space");
            return Ok(format!("{} space", literal(c)));
        }
        if let Some(Value::Array(vals)) = obj.get("enum") {
            self.primitive("space");
            let alts: Vec<String> = vals.iter().map(literal).collect();
            return Ok(format!("({}) space", alts.join(" | ")));
        }
        for key in ["anyOf", "oneOf"] {
            if let Some(Value::Array(alts)) = obj.get(key) {
                let mut rendered = Vec::new();
                for (i, alt) in alts.iter().enumerate() {
                    rendered.push(self.render(
                        alt,
                        &format!("{name}-{i}"),
                        &format!("{path}.{key}[{i}]"),
                    )?);
                }
                return Ok(format!("({})", rendered.join(" | ")));
            }
        }

        // Keywords the grammar cannot express but the validator enforces.
        for kw in [
            "pattern",
            "format",
            "minimum",
            "maximum",
            "minLength",
            "maxLength",
            "exclusiveMinimum",
            "exclusiveMaximum",
        ] {
            if obj.contains_key(kw) {
                self.warn(path, kw);
            }
        }

        let types: Vec<String> = match obj.get("type") {
            Some(Value::String(t)) => vec![t.clone()],
            Some(Value::Array(ts)) => ts
                .iter()
                .filter_map(|t| t.as_str().map(str::to_string))
                .collect(),
            None if obj.contains_key("properties") => vec!["object".to_string()],
            None if obj.contains_key("items") => vec!["array".to_string()],
            None => {
                return Err(GrammarError::Unsupported {
                    path: path.to_string(),
                    reason: "no `type`, `enum`, `const`, `$ref`, `anyOf` or `oneOf`".to_string(),
                })
            }
            Some(other) => {
                return Err(GrammarError::Unsupported {
                    path: path.to_string(),
                    reason: format!("`type` must be a string or array, got {}", short(other)),
                })
            }
        };

        let mut alts = Vec::new();
        for t in &types {
            alts.push(match t.as_str() {
                "string" | "number" | "integer" | "boolean" | "null" => self.primitive(t),
                "object" => self.render_object(obj, name, path)?,
                "array" => self.render_array(obj, name, path)?,
                other => {
                    return Err(GrammarError::Unsupported {
                        path: path.to_string(),
                        reason: format!("type `{other}`"),
                    })
                }
            });
        }
        Ok(if alts.len() == 1 {
            alts.remove(0)
        } else {
            format!("({})", alts.join(" | "))
        })
    }

    fn render_ref(&mut self, r: &str, path: &str) -> Result<String, GrammarError> {
        let (prefix, def) = r
            .strip_prefix("#/$defs/")
            .map(|d| ("$defs", d))
            .or_else(|| r.strip_prefix("#/definitions/").map(|d| ("definitions", d)))
            .ok_or_else(|| GrammarError::BadRef(r.to_string()))?;
        let rule = format!("def-{}", sanitize(def));
        if self.rules.contains_key(&rule) {
            return Ok(rule);
        }
        if self.in_progress.iter().any(|p| p == &rule) {
            return Err(GrammarError::RefCycle(r.to_string()));
        }
        let target = self
            .root
            .get(prefix)
            .and_then(|d| d.get(def))
            .ok_or_else(|| GrammarError::BadRef(r.to_string()))?;
        self.in_progress.push(rule.clone());
        let body = self.render(target, &rule, &format!("{path} -> {r}"))?;
        self.in_progress.pop();
        Ok(self.add_rule(&rule, body))
    }

    fn render_object(
        &mut self,
        obj: &serde_json::Map<String, Value>,
        name: &str,
        path: &str,
    ) -> Result<String, GrammarError> {
        self.primitive("space");
        let props = obj.get("properties").and_then(Value::as_object);
        let required: Vec<&str> = obj
            .get("required")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if matches!(
            obj.get("additionalProperties"),
            None | Some(Value::Bool(true))
        ) && props.is_some()
        {
            // Extraction wants a closed record; a free-form object would let
            // the model invent fields. We constrain to the declared ones and
            // say so.
            self.warnings.push(format!(
                "{path}: `additionalProperties` is not false; the grammar still emits only the declared properties"
            ));
        }
        let Some(props) = props.filter(|p| !p.is_empty()) else {
            return Err(GrammarError::Unsupported {
                path: path.to_string(),
                reason: "an object needs at least one declared property".to_string(),
            });
        };

        let mut parts: Vec<String> = Vec::new();
        for (i, (key, sub)) in props.iter().enumerate() {
            let kv_rule = format!("{name}-{}-kv", sanitize(key));
            let value_rule = format!("{name}-{}", sanitize(key));
            let mut value = self.render(sub, &value_rule, &format!("{path}.{key}"))?;
            let nullable_already = sub
                .get("type")
                .and_then(Value::as_array)
                .map(|a| a.iter().any(|t| t == "null"))
                .unwrap_or(false)
                || sub.get("type") == Some(&Value::String("null".to_string()));
            if !required.contains(&key.as_str()) && !nullable_already {
                let null = self.primitive("null");
                value = format!("({value} | {null})");
            }
            let body = format!(
                "{} space \":\" space {value}",
                literal(&Value::String(key.clone()))
            );
            self.add_rule(&kv_rule, body);
            parts.push(if i == 0 {
                kv_rule
            } else {
                format!("\",\" space {kv_rule}")
            });
        }
        let rule_name = if name == "root" {
            "root-object".to_string()
        } else {
            format!("{name}-object")
        };
        let body = format!("\"{{\" space {} \"}}\" space", parts.join(" "));
        Ok(self.add_rule(&rule_name, body))
    }

    fn render_array(
        &mut self,
        obj: &serde_json::Map<String, Value>,
        name: &str,
        path: &str,
    ) -> Result<String, GrammarError> {
        self.primitive("space");
        let items = obj.get("items").ok_or_else(|| GrammarError::Unsupported {
            path: path.to_string(),
            reason: "an array needs `items`".to_string(),
        })?;
        let item = self.render(items, &format!("{name}-item"), &format!("{path}[]"))?;
        let min = obj.get("minItems").and_then(Value::as_u64).unwrap_or(0) as usize;
        let max = obj
            .get("maxItems")
            .and_then(Value::as_u64)
            .map(|m| m as usize);
        if let Some(m) = max {
            if m < min {
                return Err(GrammarError::Unsupported {
                    path: path.to_string(),
                    reason: format!("maxItems {m} < minItems {min}"),
                });
            }
        }
        let sep_item = format!("\",\" space {item}");
        let body = match (min, max) {
            (0, None) => format!("\"[\" space ({item} ({sep_item})*)? \"]\" space"),
            (0, Some(0)) => "\"[\" space \"]\" space".to_string(),
            (0, Some(m)) => format!(
                "\"[\" space ({item} ({sep_item}){{0,{}}})? \"]\" space",
                m - 1
            ),
            (n, None) => format!("\"[\" space {item} ({sep_item}){{{},}} \"]\" space", n - 1),
            (n, Some(m)) => format!(
                "\"[\" space {item} ({sep_item}){{{},{}}} \"]\" space",
                n - 1,
                m - 1
            ),
        };
        let rule_name = if name == "root" {
            "root-array".to_string()
        } else {
            format!("{name}-array")
        };
        Ok(self.add_rule(&rule_name, body))
    }
}

/// A JSON value as a quoted GBNF terminal (`"\"text\""`, `"42"`, `"true"`).
fn literal(v: &Value) -> String {
    let json = v.to_string();
    let mut out = String::with_capacity(json.len() + 2);
    out.push('"');
    for c in json.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// GBNF rule names are `[a-zA-Z0-9-]+`.
fn sanitize(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if out.is_empty() {
        out.push_str("x");
    }
    out
}

fn short(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 40 {
        format!("{}…", &s[..40])
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Every rule referenced on a right-hand side must be defined — the one
    /// invariant that makes a grammar loadable at all, checked without
    /// llama.cpp by parsing our own output.
    fn assert_closed(g: &Grammar) {
        let defined: std::collections::HashSet<&str> = g
            .text
            .lines()
            .filter_map(|l| l.split(" ::= ").next())
            .map(str::trim)
            .collect();
        assert!(defined.contains("root"), "no root rule:\n{}", g.text);
        for line in g.text.lines() {
            let (_, rhs) = line.split_once(" ::= ").expect("rule line");
            // Strip quoted terminals and character classes, then every
            // bare identifier left must be a rule name.
            let mut stripped = String::new();
            let mut chars = rhs.chars().peekable();
            while let Some(c) = chars.next() {
                match c {
                    '"' => {
                        while let Some(d) = chars.next() {
                            if d == '\\' {
                                chars.next();
                            } else if d == '"' {
                                break;
                            }
                        }
                        stripped.push(' ');
                    }
                    '[' => {
                        while let Some(d) = chars.next() {
                            if d == '\\' {
                                chars.next();
                            } else if d == ']' {
                                break;
                            }
                        }
                        stripped.push(' ');
                    }
                    '{' => {
                        for d in chars.by_ref() {
                            if d == '}' {
                                break;
                            }
                        }
                        stripped.push(' ');
                    }
                    c => stripped.push(c),
                }
            }
            for ident in stripped.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')) {
                if ident.is_empty() || ident.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                assert!(
                    defined.contains(ident),
                    "undefined rule `{ident}` in: {line}\n---\n{}",
                    g.text
                );
            }
        }
    }

    #[test]
    fn a_flat_record_becomes_a_closed_object_in_schema_order() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "smoker": {"type": "boolean"}
            },
            "required": ["name", "age"],
            "additionalProperties": false
        });
        let g = to_gbnf(&schema).unwrap();
        assert_closed(&g);
        let root = g
            .text
            .lines()
            .find(|l| l.starts_with("root-object ::="))
            .unwrap();
        let name_at = root.find("root-name-kv").unwrap();
        let age_at = root.find("root-age-kv").unwrap();
        let smoker_at = root.find("root-smoker-kv").unwrap();
        assert!(
            name_at < age_at && age_at < smoker_at,
            "properties out of order: {root}"
        );
        // The optional property may be null; the required ones may not.
        assert!(g
            .text
            .contains("root-smoker-kv ::= \"\\\"smoker\\\"\" space \":\" space (boolean | null)"));
        assert!(g
            .text
            .contains("root-age-kv ::= \"\\\"age\\\"\" space \":\" space integer\n"));
        assert!(g.warnings.is_empty(), "{:?}", g.warnings);
    }

    #[test]
    fn enums_consts_and_nullable_types_render_as_alternatives() {
        let schema = json!({
            "type": "object",
            "properties": {
                "sex": {"enum": ["male", "female", "other"]},
                "version": {"const": 2},
                "note": {"type": ["string", "null"]}
            },
            "required": ["sex", "version", "note"],
            "additionalProperties": false
        });
        let g = to_gbnf(&schema).unwrap();
        assert_closed(&g);
        assert!(g
            .text
            .contains(r#"("\"male\"" | "\"female\"" | "\"other\"") space"#));
        assert!(g.text.contains(r#""2" space"#));
        assert!(g.text.contains("(string | null)"));
    }

    #[test]
    fn arrays_carry_their_bounds_as_repetitions() {
        let g = to_gbnf(&json!({"type": "array", "items": {"type": "string"}})).unwrap();
        assert_closed(&g);
        assert!(g
            .text
            .contains(r#"root-array ::= "[" space (string ("," space string)*)? "]" space"#));

        let g = to_gbnf(
            &json!({"type": "array", "items": {"type": "integer"}, "minItems": 2, "maxItems": 4}),
        )
        .unwrap();
        assert_closed(&g);
        assert!(
            g.text
                .contains(r#""[" space integer ("," space integer){1,3} "]" space"#),
            "{}",
            g.text
        );

        let g = to_gbnf(&json!({"type": "array", "items": {"type": "integer"}, "maxItems": 1}))
            .unwrap();
        assert!(g.text.contains(r#"(integer ("," space integer){0,0})?"#));
    }

    #[test]
    fn refs_become_shared_rules_and_cycles_are_refused() {
        let schema = json!({
            "$defs": {"Vital": {"type": "object", "properties": {"value": {"type": "number"}, "unit": {"type": "string"}}, "required": ["value", "unit"], "additionalProperties": false}},
            "type": "object",
            "properties": {
                "bp": {"$ref": "#/$defs/Vital"},
                "hr": {"$ref": "#/$defs/Vital"}
            },
            "required": ["bp", "hr"],
            "additionalProperties": false
        });
        let g = to_gbnf(&schema).unwrap();
        assert_closed(&g);
        assert_eq!(
            g.text.matches("def-Vital-object ::=").count(),
            1,
            "the def is rendered once:\n{}",
            g.text
        );

        let cyclic = json!({"$defs": {"Node": {"type": "object", "properties": {"next": {"$ref": "#/$defs/Node"}}, "additionalProperties": false}}, "$ref": "#/$defs/Node"});
        assert_eq!(
            to_gbnf(&cyclic),
            Err(GrammarError::RefCycle("#/$defs/Node".into()))
        );
        assert_eq!(
            to_gbnf(&json!({"$ref": "#/nope"})),
            Err(GrammarError::BadRef("#/nope".into()))
        );
    }

    #[test]
    fn unexpressible_keywords_are_warned_not_dropped_silently() {
        let g = to_gbnf(&json!({"type": "object", "properties": {"dob": {"type": "string", "format": "date"}, "age": {"type": "integer", "minimum": 0}}, "required": ["dob", "age"], "additionalProperties": false})).unwrap();
        assert_closed(&g);
        assert_eq!(g.warnings.len(), 2, "{:?}", g.warnings);
        assert!(g
            .warnings
            .iter()
            .any(|w| w.contains("$.dob") && w.contains("format")));
        assert!(g
            .warnings
            .iter()
            .any(|w| w.contains("$.age") && w.contains("minimum")));
    }

    #[test]
    fn unsupported_shapes_fail_loudly() {
        assert!(matches!(
            to_gbnf(&json!("string")),
            Err(GrammarError::NotAnObject(_))
        ));
        assert!(matches!(
            to_gbnf(&json!({"type": "object"})),
            Err(GrammarError::Unsupported { .. })
        ));
        assert!(matches!(
            to_gbnf(&json!({"type": "array"})),
            Err(GrammarError::Unsupported { .. })
        ));
        assert!(matches!(
            to_gbnf(&json!({"description": "no type"})),
            Err(GrammarError::Unsupported { .. })
        ));
        assert!(matches!(
            to_gbnf(&json!({"type": "blob"})),
            Err(GrammarError::Unsupported { .. })
        ));
    }

    #[test]
    fn keys_needing_escapes_survive_as_terminals() {
        let g = to_gbnf(&json!({"type": "object", "properties": {"say \"hi\"": {"type": "string"}}, "required": ["say \"hi\""], "additionalProperties": false})).unwrap();
        assert_closed(&g);
        // JSON escapes the quotes, then GBNF escapes the JSON's quotes again.
        assert!(
            g.text
                .contains(r#""\"say \\\"hi\\\"\"" space ":" space string"#),
            "{}",
            g.text
        );
    }

    #[test]
    fn a_clinical_note_schema_renders_whole() {
        // The shape a scribe vendor would hand us: sections, a facts list, vitals.
        let schema = json!({
            "type": "object",
            "properties": {
                "chief_complaint": {"type": "string"},
                "history": {"type": "string"},
                "vitals": {"type": "object", "properties": {"bp_systolic": {"type": "integer"}, "bp_diastolic": {"type": "integer"}, "heart_rate": {"type": "integer"}}, "additionalProperties": false},
                "medications": {"type": "array", "items": {"type": "object", "properties": {"name": {"type": "string"}, "dose": {"type": "string"}}, "required": ["name"], "additionalProperties": false}},
                "follow_up": {"anyOf": [{"type": "string"}, {"type": "null"}]}
            },
            "required": ["chief_complaint", "medications"],
            "additionalProperties": false
        });
        let g = to_gbnf(&schema).unwrap();
        assert_closed(&g);
        for rule in [
            "root",
            "root-object",
            "root-vitals-object",
            "root-medications-array",
            "root-medications-item-object",
            "string",
            "integer",
            "null",
            "space",
        ] {
            assert!(
                g.text.contains(&format!("{rule} ::=")),
                "missing {rule}:\n{}",
                g.text
            );
        }
        // The grammar never references a primitive it did not define.
        assert!(!g.text.contains("number ::="), "number is unused here");
    }
}
