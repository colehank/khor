//! Validate before consuming an interaction: invalid input must leave it answerable.
use agent_client_protocol::schema::v1::{CreateElicitationRequest, CreateElicitationResponse};
use serde_json::{json, Value};

pub fn validate_elicitation(
    request: &CreateElicitationRequest,
    response: &Value,
) -> Result<CreateElicitationResponse, String> {
    if response.to_string().len() > 128 * 1024 {
        return Err("Interaction response is too large".into());
    }
    let action = response["action"]
        .as_str()
        .ok_or("Missing interaction action")?;
    if !matches!(action, "accept" | "decline" | "cancel") {
        return Err("Unknown interaction action".into());
    }
    let parsed = serde_json::from_value(response.clone()).map_err(|e| e.to_string())?;
    if action != "accept" {
        return Ok(parsed);
    }
    let request = serde_json::to_value(request).map_err(|e| e.to_string())?;
    match request["mode"].as_str() {
        Some("url") => {
            if !response["content"].is_null() {
                return Err("URL interactions do not accept form content".into());
            }
        }
        Some("form") => {
            let empty = json!({});
            let content = if response["content"].is_null() {
                &empty
            } else {
                &response["content"]
            };
            let content = content
                .as_object()
                .ok_or("Form content must be an object")?;
            let schema = &request["requestedSchema"];
            let props = schema["properties"]
                .as_object()
                .ok_or("Unsupported form schema")?;
            if let Some(required) = schema["required"].as_array() {
                for key in required {
                    if !key.as_str().is_some_and(|k| content.contains_key(k)) {
                        return Err("A required field is missing".into());
                    }
                }
            }
            for (key, value) in content {
                let property = props
                    .get(key)
                    .ok_or_else(|| format!("Unknown field: {key}"))?;
                validate_property(property, value).map_err(|why| format!("{key}: {why}"))?;
            }
        }
        _ => return Err("Unsupported interaction mode".into()),
    }
    Ok(parsed)
}
fn validate_property(schema: &Value, value: &Value) -> Result<(), String> {
    match schema["type"].as_str() {
        Some("string") => {
            let text = value.as_str().ok_or("Expected text")?;
            let len = text.chars().count() as u64;
            if schema["minLength"].as_u64().is_some_and(|n| len < n)
                || schema["maxLength"].as_u64().is_some_and(|n| len > n)
            {
                return Err("Text length is outside the allowed range".into());
            }
            if let Some(pattern) = schema["pattern"].as_str() {
                if !regex::RegexBuilder::new(pattern)
                    .size_limit(1024 * 1024)
                    .build()
                    .map_err(|_| "Unsupported field pattern")?
                    .is_match(text)
                {
                    return Err("Text does not match the required pattern".into());
                }
            }
            if let Some(values) = schema["enum"].as_array() {
                if !values.contains(value) {
                    return Err("Choose an offered value".into());
                }
            }
            if let Some(values) = schema["oneOf"].as_array() {
                if !values.iter().any(|v| v["const"] == *value) {
                    return Err("Choose an offered value".into());
                }
            }
        }
        Some("integer" | "number") => {
            let n = value
                .as_f64()
                .filter(|n| n.is_finite())
                .ok_or("Expected a number")?;
            if schema["type"] == "integer" && n.fract() != 0.0 {
                return Err("Expected an integer".into());
            }
            if schema["minimum"].as_f64().is_some_and(|x| n < x)
                || schema["maximum"].as_f64().is_some_and(|x| n > x)
            {
                return Err("Number is outside the allowed range".into());
            }
        }
        Some("boolean") => {
            if !value.is_boolean() {
                return Err("Expected a boolean".into());
            }
        }
        Some("array") => {
            let values = value.as_array().ok_or("Expected a list")?;
            if schema["minItems"]
                .as_u64()
                .is_some_and(|n| values.len() < (n as usize))
                || schema["maxItems"]
                    .as_u64()
                    .is_some_and(|n| values.len() > (n as usize))
            {
                return Err("Select the allowed number of values".into());
            }
            for (i, value) in values.iter().enumerate() {
                if values[..i].contains(value) {
                    return Err("Duplicate selection".into());
                }
                if schema["items"]["type"] == "string" {
                    validate_property(&schema["items"], value)?;
                } else if let Some(choices) = schema["items"]["anyOf"].as_array() {
                    if !choices.iter().any(|v| v["const"] == *value) {
                        return Err("Choose an offered value".into());
                    }
                } else {
                    return Err("Unsupported list schema".into());
                }
            }
        }
        _ => return Err("Unsupported field type".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation_preserves_the_pending_request_and_rejects_invalid_values() {
        let request:CreateElicitationRequest=serde_json::from_value(json!({"mode":"form","sessionId":"s","message":"Choose","requestedSchema":{"type":"object","properties":{"count":{"type":"integer","minimum":1,"maximum":5},"choice":{"type":"string","enum":["a","b"]}},"required":["count","choice"]}})).unwrap();
        for content in [
            json!({}),
            json!({"count":1.5,"choice":"a"}),
            json!({"count":6,"choice":"a"}),
            json!({"count":2,"choice":"c"}),
            json!({"count":2,"choice":"a","extra":true}),
        ] {
            assert!(
                validate_elicitation(&request, &json!({"action":"accept","content":content}))
                    .is_err()
            );
        }
        assert!(validate_elicitation(
            &request,
            &json!({"action":"accept","content":{"count":2,"choice":"a"}})
        )
        .is_ok());
        assert!(validate_elicitation(&request, &json!({"action":"cancel"})).is_ok());
    }
}
