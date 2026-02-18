use serde_json::Value;

/// Adapt legacy payloads by mapping known old field names to the new canonical names.
/// This keeps v1 clients working while the codebase moves to the new field names.
#[allow(dead_code, clippy::collapsible_if)]
pub fn adapt_payload(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        if obj.contains_key("old_amount") && !obj.contains_key("amount") {
            if let Some(val) = obj.remove("old_amount") {
                obj.insert("amount".to_string(), val);
            }
        }
        if obj.contains_key("old_currency") && !obj.contains_key("currency") {
            if let Some(val) = obj.remove("old_currency") {
                obj.insert("currency".to_string(), val);
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn adapts_old_fields() {
        let payload = json!({"old_amount": 1000, "old_currency": "USD", "keep": true});
        let adapted = adapt_payload(payload.clone());
        assert_eq!(adapted.get("amount").unwrap(), &json!(1000));
        assert_eq!(adapted.get("currency").unwrap(), &json!("USD"));
        assert!(adapted.get("keep").is_some());
    }
}
