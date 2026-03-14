use serde_json::Value;
use uuid::Uuid;

/// Asserts that the JSON value has the structure of a V2 paginated response.
pub fn assert_paginated(json: &Value) {
    assert!(json["data"].is_array(), "expected data array in paginated response");
    assert!(json["totalCount"].is_number(), "expected totalCount number");
    assert!(json["hasMore"].is_boolean(), "expected hasMore boolean");
    // nextCursor may be null or string
    assert!(
        json["nextCursor"].is_null() || json["nextCursor"].is_string(),
        "expected nextCursor to be null or string"
    );
}

/// Asserts that the JSON value has the structure of an error response.
pub fn assert_error(json: &Value) {
    assert!(json["message"].is_string(), "expected message string in error response");
}

/// Asserts that the JSON value is a valid UUID string.
pub fn assert_uuid(value: &Value) {
    let s = value.as_str().expect("expected string for UUID");
    Uuid::parse_str(s).expect("valid UUID");
}

/// Asserts that the JSON value is a valid YYYY-MM-DD date string.
pub fn assert_date(value: &Value) {
    let s = value.as_str().expect("expected string for date");
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").expect("valid YYYY-MM-DD date");
}

/// Asserts that all object keys in the JSON are camelCase (no underscores).
/// Recursively checks nested objects and arrays.
pub fn assert_camel_case_keys(json: &Value) {
    match json {
        Value::Object(map) => {
            for (key, val) in map {
                assert!(!key.contains('_'), "key '{}' contains underscore — expected camelCase", key);
                assert_camel_case_keys(val);
            }
        }
        Value::Array(arr) => {
            for val in arr {
                assert_camel_case_keys(val);
            }
        }
        _ => {}
    }
}
