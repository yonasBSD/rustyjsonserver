use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn default_port() -> u16 {
    8080
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum RawScript {
    Inline(String),
    Ref { fref: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum RawMethodResponse {
    Script { script: RawScript },
    Response { response: Value },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawMethodDefinition {
    pub method: String,
    #[serde(flatten)]
    pub response: RawMethodResponse,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawPartialResource {
    #[serde(default)]
    pub children: Vec<RawResource>,
    #[serde(default)]
    pub methods: Vec<RawMethodDefinition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum RawResource {
    /// A resource defined by a file reference.
    Reference {
        fref: String,
        #[serde(default)]
        path: String,
    },
    /// An inline resource definition that must have a path.
    Inline {
        path: String,
        #[serde(default)]
        children: Vec<RawResource>,
        #[serde(default)]
        methods: Vec<RawMethodDefinition>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    pub resources: Vec<RawResource>,
}
