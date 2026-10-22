//! Validation over the same schema subset the grammar renders, plus the
//! normalisation that reconciles the grammar's "optional means nullable"
//! convention with standard JSON Schema.

use serde_json::Value;

/// One way a value fails its schema, addressed by a JSON path (`$.vitals.bp`).
/// The repair retry shows these to the model verbatim, so they read as plain
/// instructions rather than validator jargon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// Check `value` against `schema`. An empty list means valid.
pub fn validate(schema: &Value, value: &Value) -> Vec<Violation> {
    let mut out = Vec::new();
    check(schema, schema, value, "$", &mut out);
    out
}

fn fail(out: &mut Vec<Violation>, path: &str, message: impl Into<String>) {
    out.push(Violation {
        path: path.to_string(),
        message: message.into(),
    });
}

fn resolve<'a>(root: &'a Value, r: &str) -> Option<&'a Value> {
    let (prefix, def) = r
        .strip_prefix("#/$defs/")
        .map(|d| ("$defs", d))
        .or_else(|| r.strip_prefix("#/definitions/").map(|d| ("definitions", d)))?;
    root.get(prefix)?.get(def)
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn matches_type(v: &Value, t: &str) -> bool {
    match t {
        "number" => v.is_number(),
        "integer" => v.as_f64().map(|f| f.fract() == 0.0).unwrap_or(false),
        other => type_name(v) == other,
    }
}

fn check(root: &Value, schema: &Value, value: &Value, path: &str, out: &mut Vec<Violation>) {
    let Some(obj) = schema.as_object() else {
        fail(out, path, "schema is not an object");
        return;
    };
    if let Some(Value::String(r)) = obj.get("$ref") {
        match resolve(root, r) {
            Some(target) => check(root, target, value, path, out),
            None => fail(out, path, format!("unresolvable $ref {r}")),
        }
        return;
    }
    if let Some(c) = obj.get("const") {
        if value != c {
            fail(out, path, format!("must be exactly {c}"));
        }
        return;
    }
    if let Some(Value::Array(vals)) = obj.get("enum") {
        if !vals.contains(value) {
            let allowed: Vec<String> = vals.iter().map(Value::to_string).collect();
            fail(out, path, format!("must be one of {}", allowed.join(", ")));
        }
        return;
    }
    for key in ["anyOf", "oneOf"] {
        if let Some(Value::Array(alts)) = obj.get(key) {
            let passing = alts
                .iter()
                .filter(|alt| {
                    let mut scratch = Vec::new();
                    check(root, alt, value, path, &mut scratch);
                    scratch.is_empty()
                })
                .count();
            let ok = if key == "oneOf" {
                passing == 1
            } else {
                passing >= 1
            };
            if !ok {
                fail(
                    out,
                    path,
                    format!(
                        "matches {passing} of the {} allowed shapes under {key}",
                        alts.len()
                    ),
                );
            }
            return;
        }
    }

    let types: Vec<&str> = match obj.get("type") {
        Some(Value::String(t)) => vec![t.as_str()],
        Some(Value::Array(ts)) => ts.iter().filter_map(Value::as_str).collect(),
        _ if obj.contains_key("properties") => vec!["object"],
        _ if obj.contains_key("items") => vec!["array"],
        _ => vec![],
    };
    if !types.is_empty() && !types.iter().any(|t| matches_type(value, t)) {
        fail(
            out,
            path,
            format!("must be {}, got {}", types.join(" or "), type_name(value)),
        );
        return;
    }

    match value {
        Value::Object(map) => {
            let props = obj.get("properties").and_then(Value::as_object);
            if let Some(Value::Array(req)) = obj.get("required") {
                for r in req.iter().filter_map(Value::as_str) {
                    if !map.contains_key(r) {
                        fail(out, path, format!("missing required property \"{r}\""));
                    }
                }
            }
            let closed = obj.get("additionalProperties") == Some(&Value::Bool(false));
            for (k, v) in map {
                match props.and_then(|p| p.get(k)) {
                    Some(sub) => check(root, sub, v, &format!("{path}.{k}"), out),
                    None if closed => {
                        fail(out, &format!("{path}.{k}"), "is not a declared property")
                    }
                    None => {}
                }
            }
        }
        Value::Array(items) => {
            if let Some(min) = obj.get("minItems").and_then(Value::as_u64) {
                if (items.len() as u64) < min {
                    fail(
                        out,
                        path,
                        format!("needs at least {min} items, has {}", items.len()),
                    );
                }
            }
            if let Some(max) = obj.get("maxItems").and_then(Value::as_u64) {
                if (items.len() as u64) > max {
                    fail(
                        out,
                        path,
                        format!("allows at most {max} items, has {}", items.len()),
                    );
                }
            }
            if let Some(item_schema) = obj.get("items") {
                for (i, item) in items.iter().enumerate() {
                    check(root, item_schema, item, &format!("{path}[{i}]"), out);
                }
            }
        }
        Value::String(s) => {
            let len = s.chars().count() as u64;
            if let Some(min) = obj.get("minLength").and_then(Value::as_u64) {
                if len < min {
                    fail(out, path, format!("must be at least {min} characters"));
                }
            }
            if let Some(max) = obj.get("maxLength").and_then(Value::as_u64) {
                if len > max {
                    fail(out, path, format!("must be at most {max} characters"));
                }
            }
        }
        Value::Number(n) => {
            let f = n.as_f64().unwrap_or(0.0);
            if let Some(min) = obj.get("minimum").and_then(Value::as_f64) {
                if f < min {
                    fail(out, path, format!("must be at least {min}"));
                }
            }
            if let Some(max) = obj.get("maximum").and_then(Value::as_f64) {
                if f > max {
                    fail(out, path, format!("must be at most {max}"));
                }
            }
        }
        _ => {}
    }
}

/// Remove null-valued properties that the schema neither requires nor allows
/// to be null. The grammar lets a model write `"follow_up": null` for "not
/// mentioned" (see the crate docs); standard JSON Schema would call that a
/// type error, so the null is dropped and the property is simply absent.
/// Recurses into nested objects and arrays. Values that are not objects are
/// returned unchanged.
pub fn strip_optional_nulls(schema: &Value, value: Value) -> Value {
    strip(schema, schema, value)
}

fn strip(root: &Value, schema: &Value, value: Value) -> Value {
    let Some(obj) = schema.as_object() else {
        return value;
    };
    if let Some(Value::String(r)) = obj.get("$ref") {
        return match resolve(root, r) {
            Some(target) => strip(root, target, value),
            None => value,
        };
    }
    match value {
        Value::Object(map) => {
            let Some(props) = obj.get("properties").and_then(Value::as_object) else {
                return Value::Object(map);
            };
            let required: Vec<&str> = obj
                .get("required")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                match props.get(&k) {
                    Some(sub) => {
                        if v.is_null() && !required.contains(&k.as_str()) && !allows_null(root, sub)
                        {
                            continue;
                        }
                        out.insert(k, strip(root, sub, v));
                    }
                    None => {
                        out.insert(k, v);
                    }
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => match obj.get("items") {
            Some(item_schema) => Value::Array(
                items
                    .into_iter()
                    .map(|i| strip(root, item_schema, i))
                    .collect(),
            ),
            None => Value::Array(items),
        },
        v => v,
    }
}

fn allows_null(root: &Value, schema: &Value) -> bool {
    let mut scratch = Vec::new();
    check(root, schema, &Value::Null, "$", &mut scratch);
    scratch.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn note_schema() -> Value {
        json!({
            "$defs": {"Med": {"type": "object", "properties": {"name": {"type": "string", "minLength": 1}, "dose": {"type": "string"}}, "required": ["name"], "additionalProperties": false}},
            "type": "object",
            "properties": {
                "chief_complaint": {"type": "string"},
                "severity": {"enum": ["mild", "moderate", "severe"]},
                "heart_rate": {"type": "integer", "minimum": 20, "maximum": 250},
                "medications": {"type": "array", "items": {"$ref": "#/$defs/Med"}, "maxItems": 3},
                "follow_up": {"anyOf": [{"type": "string"}, {"type": "null"}]},
                "smoker": {"type": "boolean"}
            },
            "required": ["chief_complaint", "medications"],
            "additionalProperties": false
        })
    }

    #[test]
    fn a_conforming_note_has_no_violations() {
        let v = json!({"chief_complaint": "chest pain", "severity": "moderate", "heart_rate": 88, "medications": [{"name": "aspirin", "dose": "75 mg"}], "follow_up": null});
        assert_eq!(validate(&note_schema(), &v), vec![]);
    }

    #[test]
    fn each_violation_names_its_path_in_plain_words() {
        let v = json!({
            "severity": "bad",
            "heart_rate": 300.5,
            "medications": [{"name": ""}, {"dose": "x"}, {"name": "a"}, {"name": "b"}],
            "smoker": "yes",
            "invented": 1
        });
        let got: Vec<String> = validate(&note_schema(), &v)
            .iter()
            .map(ToString::to_string)
            .collect();
        let expect = [
            "$: missing required property \"chief_complaint\"",
            "$.severity: must be one of \"mild\", \"moderate\", \"severe\"",
            "$.heart_rate: must be integer, got number",
            "$.medications: allows at most 3 items, has 4",
            "$.medications[0].name: must be at least 1 characters",
            "$.medications[1]: missing required property \"name\"",
            "$.smoker: must be boolean, got string",
            "$.invented: is not a declared property",
        ];
        for e in expect {
            assert!(got.contains(&e.to_string()), "missing {e:?} in {got:#?}");
        }
        assert_eq!(got.len(), expect.len(), "{got:#?}");
    }

    #[test]
    fn integers_accept_whole_floats_and_bounds_are_enforced() {
        let s = json!({"type": "integer", "minimum": 0, "maximum": 10});
        assert!(validate(&s, &json!(5.0)).is_empty());
        assert_eq!(validate(&s, &json!(11))[0].message, "must be at most 10");
        assert_eq!(validate(&s, &json!(-1))[0].message, "must be at least 0");
        assert_eq!(
            validate(&s, &json!(1.5))[0].message,
            "must be integer, got number"
        );
    }

    #[test]
    fn one_of_demands_exactly_one_match() {
        let s = json!({"oneOf": [{"type": "integer"}, {"type": "number"}]});
        // 2 is both an integer and a number: zero or two matches both fail oneOf.
        assert_eq!(
            validate(&s, &json!(2))[0].message,
            "matches 2 of the 2 allowed shapes under oneOf"
        );
        assert!(validate(&s, &json!(2.5)).is_empty());
        let any = json!({"anyOf": [{"type": "integer"}, {"type": "number"}]});
        assert!(validate(&any, &json!(2)).is_empty());
        assert_eq!(
            validate(&any, &json!("x"))[0].message,
            "matches 0 of the 2 allowed shapes under anyOf"
        );
    }

    #[test]
    fn nullable_type_arrays_and_const_work() {
        assert!(validate(&json!({"type": ["string", "null"]}), &json!(null)).is_empty());
        assert_eq!(
            validate(&json!({"type": ["string", "null"]}), &json!(1))[0].message,
            "must be string or null, got integer"
        );
        assert_eq!(
            validate(&json!({"const": 2}), &json!(3))[0].message,
            "must be exactly 2"
        );
        assert_eq!(
            validate(&json!({"$ref": "#/nope"}), &json!(3))[0].message,
            "unresolvable $ref #/nope"
        );
    }

    #[test]
    fn optional_nulls_are_stripped_but_allowed_nulls_stay() {
        let v = json!({
            "chief_complaint": "cough",
            "severity": null,
            "heart_rate": null,
            "follow_up": null,
            "medications": [{"name": "x", "dose": null}]
        });
        let stripped = strip_optional_nulls(&note_schema(), v);
        assert_eq!(
            stripped,
            json!({"chief_complaint": "cough", "follow_up": null, "medications": [{"name": "x"}]})
        );
        // What the grammar lets the model write now validates as standard JSON Schema.
        assert_eq!(validate(&note_schema(), &stripped), vec![]);
        // A REQUIRED property set to null is kept, so the validator can report it.
        let v = json!({"chief_complaint": null, "medications": []});
        let stripped = strip_optional_nulls(&note_schema(), v);
        assert_eq!(
            validate(&note_schema(), &stripped)[0].message,
            "must be string, got null"
        );
    }
}
