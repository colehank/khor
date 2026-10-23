//! Native control/initialize model metadata; no model-name heuristics.
use agent_client_protocol::schema::v1::SessionConfigOption;
use serde_json::{Value, json};
#[derive(Default, Clone)]
pub(super) struct Config {
    pub models: Vec<Value>,
    pub model: Option<String>,
    pub effort: Option<String>,
}
impl Config {
    pub fn options(&self) -> Vec<SessionConfigOption> {
        if self.models.is_empty() {
            return vec![];
        }
        let model = self.model.as_deref().unwrap_or("default");
        let mut choices:Vec<Value>=self.models.iter().filter_map(|v|Some(json!({"value":v["value"].as_str()?,"name":v["displayName"].as_str().or(v["value"].as_str())?,"description":v["description"]}))).collect();
        if !choices.iter().any(|v| v["value"] == model) {
            choices.insert(
                0,
                json!({"value":model,"name":if model=="default"{"Default"}else{model}}),
            );
        }
        let mut options = vec![
            json!({"id":"model","name":"Model","category":"model","type":"select","currentValue":model,"options":choices}),
        ];
        if let Some(m) = self
            .models
            .iter()
            .find(|v| v["value"] == model || v["resolvedModel"] == model)
        {
            let mut levels = vec![json!({"value":"default","name":"Default"})];
            levels.extend(
                m["supportedEffortLevels"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str().map(|s| json!({"value":s,"name":s}))),
            );
            if levels.len() > 1 {
                options.push(json!({"id":"thought_level","name":"Thinking","category":"thought_level","type":"select","currentValue":self.effort.as_deref().unwrap_or("default"),"options":levels}));
            }
        }
        options
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect()
    }
    pub fn change(&mut self, id: &str, value: &Value) -> Result<Value, String> {
        let choices: Vec<Value> = self
            .options()
            .into_iter()
            .map(|v| serde_json::to_value(v).unwrap())
            .collect();
        let option = choices
            .iter()
            .find(|v| v["id"] == id)
            .ok_or("Unknown session option")?;
        if !option["options"]
            .as_array()
            .is_some_and(|xs| xs.iter().any(|v| v["value"] == *value))
        {
            return Err("Choose a value offered by this model".into());
        }
        match id {
            "model" => {
                self.model = value.as_str().map(str::to_owned);
                self.effort = None;
                Ok(
                    json!({"model":if value=="default"{Value::Null}else{value.clone()},"effortLevel":null}),
                )
            }
            "thought_level" => {
                self.effort = value
                    .as_str()
                    .filter(|v| *v != "default")
                    .map(str::to_owned);
                Ok(json!({"effortLevel":self.effort}))
            }
            _ => Err("Unknown session option".into()),
        }
    }
}
