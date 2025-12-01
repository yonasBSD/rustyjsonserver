use std::collections::HashMap;

use serde_json::Value;
use tracing::{debug};

use crate::rjscript::{
    self,
    ast::{block::Block, position::Position},
    parser::parser,
};

use super::resolved::{ResolvedConfig, ResolvedMethodResponse, ResolvedResource};

#[derive(Clone, Debug)]
pub enum CompiledMethodResponse {
    Script { script: Block },
    Response { status: u16, body: Value },
}

#[derive(Clone, Debug)]
pub struct CompiledMethodDefinition {
    pub method: String,
    pub response: CompiledMethodResponse,
}

#[derive(Clone, Debug)]
pub struct CompiledResource {
    path: String,
    children: Vec<CompiledResource>,
    methods: Vec<CompiledMethodDefinition>,
}

impl CompiledResource {
    pub fn methods_map(&self) -> HashMap<String, CompiledMethodResponse> {
        self.methods
            .iter()
            .map(|def| (def.method.clone(), def.response.clone()))
            .collect()
    }

    pub fn get_path(&self) -> &str {
        self.path.as_str()
    }

    pub fn children(&self) -> Vec<CompiledResource> {
        self.children.clone()
    }
}

pub struct CompiledConfig {
    pub port: u16,
    pub resources: Vec<CompiledResource>,
}

fn compile_method_response(
    response: ResolvedMethodResponse,
) -> Result<CompiledMethodResponse, String> {
    debug!("Compiling method response: {response:?}");
    match response {
        ResolvedMethodResponse::Script { script } => {
            match parser::parse_script(&script) {
                Ok(block) => {
                    // Run lints + transforms
                    let prep = rjscript::preprocess::preprocess(block.stmts);

                    if !prep.errors.is_empty() {
                        for e in &prep.errors {
                            eprintln!("{e}");
                        }
                        return Err("lint errors".into());
                    }

                    let processed_block = Block::new(prep.stmts, Position::UNKNOWN);

                    Ok(CompiledMethodResponse::Script {
                        script: processed_block,
                    })
                }
                Err(err) => Err(format!("Failed to parse script: {}", err)),
            }
        }
        ResolvedMethodResponse::Response { response } => {
            match response {
                Value::Object(mut map) => {
                    let body = map.remove("body").ok_or_else(|| {
                        "response object must contain a 'body' field".to_string()
                    })?;

                    let status = match map.remove("status") {
                        Some(Value::Number(n)) => n
                            .as_u64()
                            .and_then(|v| u16::try_from(v).ok())
                            .ok_or_else(|| "response.status must be a valid u16".to_string())?,
                        Some(_) => {
                            return Err("response.status must be a number".to_string());
                        }
                        None => 200,
                    };

                    Ok(CompiledMethodResponse::Response { status, body })
                }
                _ => Err(
                    "response must be an object with at least a 'body' field".to_string(),
                ),
            }
        }
    }
}

fn compile_resource(resource: ResolvedResource) -> Result<CompiledResource, String> {
    debug!(path = %resource.path, "Compiling resource");
    // Compile child resources recursively.
    let compiled_children = resource
        .children
        .into_iter()
        .map(compile_resource)
        .collect::<Result<Vec<_>, String>>()?;

    // Compile each method in the resource.
    let mut compiled_methods = Vec::with_capacity(resource.methods.len());
    for method in resource.methods {
        let compiled_resp = compile_method_response(method.response)?;
        compiled_methods.push(CompiledMethodDefinition {
            method: method.method,
            response: compiled_resp,
        });
    }

    Ok(CompiledResource {
        path: resource.path,
        children: compiled_children,
        methods: compiled_methods,
    })
}

pub fn compile_config(resolved: ResolvedConfig) -> Result<CompiledConfig, String> {
    let compiled_resources = resolved
        .resources
        .into_iter()
        .map(compile_resource)
        .collect::<Result<Vec<_>, String>>()?;

    Ok(CompiledConfig {
        port: resolved.port,
        resources: compiled_resources,
    })
}
