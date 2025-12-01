use serde_json;
use std::{env, fs, io::ErrorKind, path::Path};

use super::{raw::{RawConfig, RawMethodResponse, RawPartialResource, RawResource, RawScript}, resolved::{ResolvedConfig, ResolvedMethodDefinition, ResolvedMethodResponse, ResolvedResource}};


pub fn get_config_path_cwd(config_arg: &str) -> String {
    let config_path = Path::new(config_arg);
    if config_path.is_absolute() {
        config_arg.to_string()
    } else {
        // Get the current working directory and join the relative path.
        let cwd =
            env::current_dir().unwrap_or_else(|e| panic!("Failed to get current directory: {}", e));
        cwd.join(config_path)
            .to_str()
            .unwrap_or_else(|| panic!("Invalid config path"))
            .to_string()
    }
}

/// Resolves a given reference (for a children_file or script) relative to the provided base directory.
pub fn resolve_path(reference: &str, base: &Path) -> String {
    let ref_path = Path::new(reference);
    if ref_path.is_absolute() {
        reference.to_string()
    } else {
        base.join(ref_path)
            .to_str()
            .unwrap_or(reference)
            .to_string()
    }
}

/// Loads a script from a file given its reference.
pub fn load_script_from_ref(script_ref: &str, root_folder: &Path) -> Result<String, String> {
    let script_path = resolve_path(script_ref, root_folder);
    fs::read_to_string(script_path.trim())
        .map_err(|e| format!("Error reading script file {}: {}", script_path, e))
}

fn resolve_method_response(mut raw: RawMethodResponse, root_folder: &Path) -> Result<ResolvedMethodResponse, String> {
    // If it's a script response and we have a reference, inline it.
    if let RawMethodResponse::Script { ref mut script } = raw {
        if let RawScript::Ref { ref fref } = script {
            let script_content = load_script_from_ref(&fref, root_folder)?;
            *script = RawScript::Inline(script_content);
        }
    }
    
    // Now that we assume the script is inline, extract it.
    match raw {
        RawMethodResponse::Script { script } => {
            if let RawScript::Inline(s) = script {
                Ok(ResolvedMethodResponse::Script { script: s })
            } else {
                unreachable!("All script references should have been inlined")
            }
        }
        RawMethodResponse::Response { response } => {
            Ok(ResolvedMethodResponse::Response { response })
        }
    }
}

fn inline_resource(resource: RawResource, root_folder: &Path) -> Result<ResolvedResource, String> {
    match resource {
        RawResource::Inline { path, children, methods } => {
            // Process children: inline and convert each child.
            let resolved_children = children.into_iter()
                .map(|child| inline_resource(child, root_folder))
                .collect::<Result<Vec<_>, String>>()?;

            // Process each method to inline any script references.
            let mut resolved_methods = Vec::with_capacity(methods.len());
            for method in methods {
                let resolved_method = ResolvedMethodDefinition {
                    method: method.method,
                    response: resolve_method_response(method.response, root_folder)?,
                };
                resolved_methods.push(resolved_method);
            }

            Ok(ResolvedResource {
                path,
                children: resolved_children,
                methods: resolved_methods,
            })
        }
        RawResource::Reference { fref, path: override_path } => {
            // Resolve the external reference file.
            let external_path_str = resolve_path(&fref, root_folder);
            let external_path = Path::new(&external_path_str);
            let file_content = fs::read_to_string(&external_path)
                .map_err(|e| format!("Error reading reference file {}: {}", external_path_str, e))?;
            // Deserialize the external file as a PartialResource.
            let partial: RawPartialResource = serde_json::from_str(&file_content)
                .map_err(|e| format!("Failed to parse external resource {}: {}", external_path_str, e))?;
            // Ensure that an override path is provided.
            if override_path.trim().is_empty() {
                return Err(format!("Reference file {} must provide a non-empty override path.", external_path_str));
            }
            // Combine the override path with the partial resource to produce an inline resource.
            let inlined_resource = RawResource::Inline {
                path: override_path,
                children: partial.children,
                methods: partial.methods,
            };
            // Use the external file's directory as the new base for resolving further references.
            let new_root = external_path.parent().unwrap_or(root_folder);
            // Recursively resolve the inline resource.
            inline_resource(inlined_resource, new_root)
        }
    }
}

/// Inlines external references throughout the configuration.
pub fn resolve_config_references(config: RawConfig, root_folder: &Path) -> Result<ResolvedConfig, String> {
    let resolved_resources = config.resources.into_iter()
        .map(|resource| inline_resource(resource, root_folder))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(ResolvedConfig {
        port: config.port,
        resources: resolved_resources,
    })
}

pub fn load_config(path: &str) -> Result<RawConfig, String> {
    let file_content = fs::read_to_string(path).map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            format!("Configuration file '{}' not found.", path)
        } else {
            format!("Failed to read configuration file '{}': {}", path, e)
        }
    })?;
    serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to parse configuration file '{}': {}", path, e))
}
