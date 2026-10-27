//! The schema as a small model should see it: a filled-in example with
//! placeholders, not the schema itself. On the first real run (PriMock57
//! day1_consultation01, Qwen2.5 0.5B, 2026-10-27) the raw schema in the prompt
//! was echoed straight back, `$comment` and all, until the token budget ran
//! out. A shape is shorter, has nothing to copy, and reads as the answer's
//! outline. The full schema is still what the output is validated against.

use serde_json::Value;

/// Render `schema` as an example JSON document with placeholder values.
pub fn shape(schema: &Value) -> String {
    let mut out = String::new();
    render(schema, schema, &mut out, 0);
    out
}

fn resolve<'a>(root: &'a Value, r: &str) -> Option<&'a Value> {
    let (prefix, def) = r
        .strip_prefix("#/$defs/")
        .map(|d| ("$defs", d))
        .or_else(|| r.strip_prefix("#/definitions/").map(|d| ("definitions", d)))?;
    root.get(prefix)?.get(def)
}

fn render(root: &Value, schema: &Value, out: &mut String, depth: usize) {
    let Some(obj) = schema.as_object() else {
        out.push_str("null");
        return;
    };
    if let Some(Value::String(r)) = obj.get("$ref") {
        match resolve(root, r) {
            Some(t) if depth < 8 => render(root, t, out, depth + 1),
            _ => out.push_str("null"),
        }
        return;
    }
    if let Some(c) = obj.get("const") {
        out.push_str(&c.to_string());
        return;
    }
    if let Some(Value::Array(vals)) = obj.get("enum") {
        let alts: Vec<String> = vals
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| v.to_string())
            })
            .collect();
        out.push_str(&format!("\"<one of: {}>\"", alts.join(" | ")));
        return;
    }
    for key in ["anyOf", "oneOf"] {
        if let Some(Value::Array(alts)) = obj.get(key) {
            // Show the first non-null alternative; "or null" is said in words.
            let nullable = alts
                .iter()
                .any(|a| a.get("type") == Some(&Value::String("null".into())));
            if let Some(first) = alts
                .iter()
                .find(|a| a.get("type") != Some(&Value::String("null".into())))
            {
                let mut inner = String::new();
                render(root, first, &mut inner, depth + 1);
                out.push_str(&if nullable { or_null(&inner) } else { inner });
                return;
            }
        }
    }
    let types: Vec<&str> = match obj.get("type") {
        Some(Value::String(t)) => vec![t.as_str()],
        Some(Value::Array(ts)) => ts.iter().filter_map(Value::as_str).collect(),
        _ if obj.contains_key("properties") => vec!["object"],
        _ if obj.contains_key("items") => vec!["array"],
        _ => vec!["string"],
    };
    let nullable = types.contains(&"null");
    let main = types
        .iter()
        .copied()
        .find(|t| *t != "null")
        .unwrap_or("null");
    let hint = obj.get("description").and_then(Value::as_str);
    let start = out.len();
    match main {
        "object" => {
            let indent = "  ".repeat(depth + 1);
            out.push_str("{\n");
            if let Some(props) = obj.get("properties").and_then(Value::as_object) {
                let n = props.len();
                for (i, (k, sub)) in props.iter().enumerate() {
                    out.push_str(&indent);
                    out.push_str(&Value::String(k.clone()).to_string());
                    out.push_str(": ");
                    render(root, sub, out, depth + 1);
                    if i + 1 < n {
                        out.push(',');
                    }
                    out.push('\n');
                }
            }
            out.push_str(&"  ".repeat(depth));
            out.push('}');
        }
        "array" => {
            // One example item and nothing else: a model that copies ", ..."
            // produces JSON that cannot be parsed (seen on the first real run).
            out.push('[');
            if let Some(items) = obj.get("items") {
                render(root, items, out, depth + 1);
            }
            out.push(']');
        }
        "integer" => out.push_str(
            &hint
                .map(|h| format!("\"<integer: {h}>\""))
                .unwrap_or_else(|| "\"<integer>\"".into()),
        ),
        "number" => out.push_str(
            &hint
                .map(|h| format!("\"<number: {h}>\""))
                .unwrap_or_else(|| "\"<number>\"".into()),
        ),
        "boolean" => out.push_str("\"<true or false>\""),
        "null" => out.push_str("null"),
        _ => out.push_str(
            &hint
                .map(|h| format!("\"<{h}>\""))
                .unwrap_or_else(|| "\"<text>\"".into()),
        ),
    }
    if nullable && main != "null" {
        let placeholder = out.split_off(start);
        out.push_str(&or_null(&placeholder));
    }
}

/// `"<integer>"` → `"<integer or null>"`; anything else gets ` or null` inside
/// the placeholder quotes when it is one, or is left alone.
fn or_null(placeholder: &str) -> String {
    match placeholder
        .strip_prefix("\"<")
        .and_then(|p| p.strip_suffix(">\""))
    {
        Some(inner) => format!("\"<{inner} or null>\""),
        None => placeholder.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_shape_is_an_outline_with_placeholders_not_the_schema() {
        let schema = json!({
            "$comment": "must not appear",
            "type": "object",
            "properties": {
                "facts": {"type": "array", "maxItems": 40, "items": {"type": "object", "properties": {
                    "text": {"type": "string", "minLength": 3, "description": "one statement"},
                    "group": {"enum": ["plan", "other"]}
                }, "required": ["text", "group"], "additionalProperties": false}},
                "weeks": {"type": ["integer", "null"], "minimum": 0},
                "referred": {"type": "boolean"},
                "note": {"anyOf": [{"type": "string"}, {"type": "null"}]}
            },
            "required": ["facts"],
            "additionalProperties": false
        });
        let s = shape(&schema);
        assert!(
            !s.contains("$comment") && !s.contains("maxItems") && !s.contains("required"),
            "{s}"
        );
        assert!(s.contains("\"facts\": [{"), "{s}");
        assert!(
            !s.contains("..."),
            "nothing a copying model would break the JSON with: {s}"
        );
        assert!(!s.contains("/*"), "{s}");
        assert!(s.contains("\"text\": \"<one statement>\""), "{s}");
        assert!(s.contains("\"group\": \"<one of: plan | other>\""), "{s}");
        assert!(s.contains("\"weeks\": \"<integer or null>\""), "{s}");
        assert!(s.contains("\"referred\": \"<true or false>\""), "{s}");
        assert!(s.contains("\"note\": \"<text or null>\""), "{s}");
        assert!(s.ends_with('}'));
        assert!(
            s.len() < schema.to_string().len(),
            "the shape must be shorter than the schema"
        );
    }

    #[test]
    fn refs_consts_and_cycles_are_handled() {
        let schema = json!({"$defs": {"V": {"type": "object", "properties": {"v": {"type": "number"}}}}, "type": "object", "properties": {"bp": {"$ref": "#/$defs/V"}, "version": {"const": 2}}});
        let s = shape(&schema);
        assert!(
            s.contains("\"v\": \"<number>\"") && s.contains("\"version\": 2"),
            "{s}"
        );
        let cyclic = json!({"$defs": {"N": {"type": "object", "properties": {"next": {"$ref": "#/$defs/N"}}}}, "$ref": "#/$defs/N"});
        let s = shape(&cyclic);
        assert!(s.contains("null"), "recursion stops: {s}");
    }
}
