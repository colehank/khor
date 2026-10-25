//! Values come from app-server model/list, including provider-specific effort names.
use agent_client_protocol::schema::v1::SessionConfigOption;
use serde_json::{Value, json};
#[derive(Default)]
pub(super) struct Config {
    pub models: Vec<Value>,
    pub model: Option<String>,
    pub effort: Option<String>,
}
impl Config {
    pub fn options(&self) -> Vec<SessionConfigOption> {
        let Some(model) = &self.model else {
            return vec![];
        };
        let mut values:Vec<Value>=self.models.iter().filter_map(|v|Some(json!({"value":v["model"].as_str()?,"name":v["displayName"].as_str().or(v["model"].as_str())?,"description":v["description"]}))).collect();
        if !values.iter().any(|v| v["value"] == *model) {
            values.insert(0, json!({"value":model,"name":model}));
        }
        let mut options = vec![
            json!({"id":"model","name":"Model","category":"model","type":"select","currentValue":model,"options":values}),
        ];
        if let Some(selected) = self.models.iter().find(|v| v["model"] == *model) {
            let levels:Vec<Value>=selected["supportedReasoningEfforts"].as_array().into_iter().flatten().filter_map(|v|Some(json!({"value":v["reasoningEffort"].as_str()?,"name":v["reasoningEffort"],"description":v["description"]}))).collect();
            if !levels.is_empty() {
                let current = self
                    .effort
                    .as_deref()
                    .filter(|e| levels.iter().any(|v| v["value"] == *e))
                    .or(selected["defaultReasoningEffort"].as_str())
                    .unwrap_or("");
                options.push(json!({"id":"thought_level","name":"Thinking","category":"thought_level","type":"select","currentValue":current,"options":levels}));
            }
        }
        options
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect()
    }
    pub fn set(&mut self, id: &str, value: &Value) -> Result<(), String> {
        let options: Vec<Value> = self
            .options()
            .into_iter()
            .map(|v| serde_json::to_value(v).unwrap())
            .collect();
        let option = options
            .iter()
            .find(|o| o["id"] == id)
            .ok_or("Unknown session option")?;
        if !option["options"]
            .as_array()
            .is_some_and(|xs| xs.iter().any(|x| x["value"] == *value))
        {
            return Err("Choose a value offered by this model".into());
        }
        match id {
            "model" => {
                self.model = value.as_str().map(str::to_owned);
                self.effort = self
                    .models
                    .iter()
                    .find(|v| v["model"] == *value)
                    .and_then(|v| v["defaultReasoningEffort"].as_str())
                    .map(str::to_owned);
            }
            "thought_level" => self.effort = value.as_str().map(str::to_owned),
            _ => return Err("Unknown session option".into()),
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_change_replaces_efforts_from_the_native_catalog() {
        let mut c = Config {
            model: Some("a".into()),
            effort: Some("deep".into()),
            models: vec![
                json!({"model":"a","displayName":"A","defaultReasoningEffort":"deep","supportedReasoningEfforts":[{"reasoningEffort":"deep"}]}),
                json!({"model":"b","displayName":"B","defaultReasoningEffort":"brief","supportedReasoningEfforts":[{"reasoningEffort":"brief"}]}),
            ],
        };
        assert!(c.set("thought_level", &json!("invented")).is_err());
        c.set("model", &json!("b")).unwrap();
        assert_eq!(c.effort.as_deref(), Some("brief"));
        assert!(c.set("thought_level", &json!("deep")).is_err());
        let v = serde_json::to_value(c.options()).unwrap();
        assert_eq!(v[1]["options"][0]["value"], "brief");
    }
}
