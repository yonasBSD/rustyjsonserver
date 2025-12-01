use std::{path::{Path, PathBuf}, sync::{Arc, RwLock}};
use super::resolver::{load_config, resolve_config_references};
use super::compiled::compile_config;
use crate::http::router::{get_routes_from_config, RoutesData};

#[derive(Clone)]
pub struct ConfigManager {
    config_path: String,
    root_folder: PathBuf,
    routes: Arc<RwLock<Option<RoutesData>>>,
    port: u16,
}

impl ConfigManager {
    /// Initial load + compile
    pub fn new(config_path: String) -> Result<Self, String> {
        let path = Path::new(&config_path);
        let root_folder = path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let raw = load_config(&config_path)?;
        let resolved = resolve_config_references(raw, &root_folder)?;
        let compiled = compile_config(resolved)?;

        let initial_routes = get_routes_from_config(&compiled, &root_folder);
        let port = compiled.port;
        let routes = Arc::new(RwLock::new(Some(initial_routes)));

        Ok(ConfigManager { config_path, root_folder, routes, port })
    }

    /// Reload on file change
    pub fn reload(&self) -> Result<(), String> {
        let raw = load_config(&self.config_path)?;
        let resolved = resolve_config_references(raw, &self.root_folder)?;
        let compiled = compile_config(resolved)?;
        let new_routes = get_routes_from_config(&compiled, &self.root_folder);

        *self.routes.write().unwrap() = Some(new_routes);
        Ok(())
    }

    pub fn routes_handle(&self) -> Arc<RwLock<Option<RoutesData>>> {
        Arc::clone(&self.routes)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn root_folder(&self) -> &PathBuf {
        &self.root_folder
    }
}