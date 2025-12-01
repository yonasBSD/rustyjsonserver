use std::collections::HashMap;
use std::path::Path;

use crate::config::compiled::{CompiledConfig, CompiledMethodResponse, CompiledResource};

#[derive(Debug, Clone)]
pub struct RouteNode {
    pub methods: HashMap<String, CompiledMethodResponse>,
    pub static_children: HashMap<String, Box<RouteNode>>,
    pub dynamic_child: Option<(String, Box<RouteNode>)>,
}

impl RouteNode {
    pub fn new() -> Self {
        RouteNode {
            methods: HashMap::new(),
            static_children: HashMap::new(),
            dynamic_child: None,
        }
    }
}

#[derive(Clone)]
pub struct RoutesData {
    pub static_routes: StaticRoutes,
    pub dynamic_root: RouteNode,
}

pub type StaticRoutes = HashMap<String, HashMap<String, CompiledMethodResponse>>;

fn match_static_route(
    static_routes: &StaticRoutes,
    path: &str,
    method: &str,
) -> Option<CompiledMethodResponse> {
    if let Some(methods) = static_routes.get(path) {
        if let Some(resp) = methods.get(method) {
            return Some(resp.clone());
        }
    }
    None
}

fn match_dynamic_route(
    root: &RouteNode,
    path_segments: &[&str],
    method: &str,
) -> Option<(CompiledMethodResponse, HashMap<String, String>)> {
    let mut current = root;
    let mut route_params = HashMap::new();

    for seg in path_segments {
        if let Some(child_node) = current.static_children.get(*seg) {
            current = child_node;
        } else if let Some((param_name, dynamic_node)) = &current.dynamic_child {
            route_params.insert(param_name.clone(), seg.to_string());
            current = dynamic_node;
        } else {
            return None;
        }
    }

    if let Some(resp) = current.methods.get(method) {
        Some((resp.clone(), route_params))
    } else {
        None
    }
}

pub fn find_route(
    static_routes: &StaticRoutes,
    dynamic_root: &RouteNode,
    raw_path: &str,
    method: &str,
) -> Option<(CompiledMethodResponse, HashMap<String, String>)> {
    if let Some(resp) = match_static_route(static_routes, raw_path, method) {
        return Some((resp, HashMap::new()));
    }

    let segments: Vec<&str> = raw_path.split('/').filter(|s| !s.is_empty()).collect();
    match match_dynamic_route(dynamic_root, &segments, method) {
        Some((resp, route_params)) => Some((resp, route_params)),
        None => None,
    }
}

fn is_dynamic_segment(segment: &str) -> bool {
    segment.starts_with(':') && segment.len() > 1
}

fn insert_dynamic_path(
    root: &mut RouteNode,
    path_segments: &[&str],
    methods: &std::collections::HashMap<String, CompiledMethodResponse>,
) {
    let mut current = root;

    for seg in path_segments {
        if is_dynamic_segment(seg) {
            let param_name = seg.trim_start_matches(':').to_string();
            current = current
                .dynamic_child
                .get_or_insert_with(|| (param_name.clone(), Box::new(RouteNode::new())))
                .1
                .as_mut();
        } else {
            current = current
                .static_children
                .entry(seg.to_string())
                .or_insert_with(|| Box::new(RouteNode::new()))
                .as_mut();
        }
    }
    for (method, resp) in methods {
        current.methods.insert(method.clone(), resp.clone());
    }
}

fn insert_static_path(
    static_routes: &mut StaticRoutes,
    full_path: &str,
    methods: &std::collections::HashMap<String, CompiledMethodResponse>,
) {
    static_routes.insert(full_path.to_string(), methods.clone());
}

fn compute_full_route_path(parent_path: &str, resource: &CompiledResource) -> String {
    if parent_path.is_empty() {
        resource.get_path().to_string()
    } else {
        format!("{}/{}", parent_path, resource.get_path())
    }
}

fn process_route_insertion(
    static_routes: &mut StaticRoutes,
    dynamic_root: &mut RouteNode,
    full_path: &str,
    methods_map: &std::collections::HashMap<String, CompiledMethodResponse>,
) {
    let segments: Vec<&str> = full_path.split('/').filter(|s| !s.is_empty()).collect();
    let has_dynamic = segments.iter().any(|seg| seg.starts_with(':'));

    if has_dynamic {
        insert_dynamic_path(dynamic_root, &segments, methods_map);
    } else {
        let path_with_slash = format!("/{}", segments.join("/"));
        insert_static_path(static_routes, &path_with_slash, methods_map);
    }
}

fn populate_routes(
    static_routes: &mut StaticRoutes,
    dynamic_root: &mut RouteNode,
    resource: &CompiledResource,
    parent_path: &str,
    root_folder: &std::path::Path,
) {
    // Compute the full path for the current resource.
    let full_path = compute_full_route_path(parent_path, resource);

    // Build the methods map from the resource.
    let methods_map = resource.methods_map();

    // Insert the route into either the static or dynamic route tree.
    process_route_insertion(static_routes, dynamic_root, &full_path, &methods_map);

    // Process inline children only if the resource is inline.
    for child in resource.children() {
        populate_routes(static_routes, dynamic_root, &child, &full_path, root_folder);
    }
}

pub fn get_routes_from_config(config: &CompiledConfig, root_folder: &Path) -> RoutesData {
    let mut static_routes: StaticRoutes = HashMap::new();
    let mut dynamic_root = RouteNode::new();

    for resource in &config.resources {
        populate_routes(
            &mut static_routes,
            &mut dynamic_root,
            resource,
            "",
            root_folder,
        );
    }

    RoutesData {
        static_routes,
        dynamic_root,
    }
}
