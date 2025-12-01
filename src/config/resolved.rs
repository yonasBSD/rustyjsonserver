use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResolvedMethodResponse {
    Script { script: String },
    Response { response: Value },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResolvedMethodDefinition {
    pub method: String,
    #[serde(flatten)]
    pub response: ResolvedMethodResponse,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResolvedResource {
    pub path: String,
    #[serde(default)]
    pub children: Vec<ResolvedResource>,
    #[serde(default)]
    pub methods: Vec<ResolvedMethodDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolvedConfig {
    pub port: u16,
    pub resources: Vec<ResolvedResource>,
}
