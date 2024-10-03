use std::path::PathBuf;
use std::sync::Arc;
use sws_mod_api::*;
use crate::handler::RequestHandlerOpts;
use glob::glob;
use dlopen::wrapper::{Container, WrapperApi};
use dlopen_derive::WrapperApi;
use http::Request;
use hyper::Body;


#[derive(WrapperApi)]
pub struct ModuleApi {
    filter_request: fn(request: &ModuleRequest) -> bool,
    handle_request: fn(request: &ModuleRequest) -> ModuleResult,
}

pub type LoadedMod = Arc<Container<ModuleApi>>;

impl ModuleApi {
    fn filter_request_impl(&self, request: &Request<Body>) -> bool {
        self.filter_request(request)
    }
    fn handle_request_impl(&self, request: &Request<Body>) -> ModuleResult {
        self.handle_request(request)
    }
}

pub fn init(mods_dir: PathBuf, handler_opts: &mut RequestHandlerOpts) {
    let mut mods = vec![];
    if mods_dir.is_dir() {
        server_info!("Loading mods...");
        let pattern = mods_dir
            .join("**")
            .join(
                if cfg!(target_os = "macos") {
                    "*.rlib"
                } else if cfg!(target_os = "windows") {
                    "*.dll"
                } else {
                    "*.so"
                }
            );
        if let Some(str) = pattern.to_str() {
            if let Ok(paths) = glob(str) {
                for entry in paths {
                    if let Ok(path) = entry {
                        let path_clone = path.clone();
                        let path_clone = path_clone.display();
                        server_info!("Loading mod from path: '{path_clone}'");
                        let container_result: Result<Container<ModuleApi>, _> = unsafe { Container::load(path.clone()) };
                        match container_result {
                            Ok(container) => mods.push(Arc::new(container)),
                            Err(error) => {
                                let err = error.to_string();
                                let path = path.clone();
                                let path = path.display();
                                server_warn!("Failed to load mod: '{path}', {err}");
                            }
                        }
                    }
                }
            }
        }
    }
    let count = mods.len();
    server_info!("Loaded {count} mods");
    handler_opts.mods = mods;
}

pub fn pre_process(opts: &RequestHandlerOpts, req: &Request<Body>) -> Option<ModuleResult> {
    let mods: &Vec<Arc<Container<ModuleApi>>> = &opts.mods.to_vec();
    for module in mods {
        let mod_clone = module.clone();
        if !mod_clone.filter_request_impl(req) {
            continue;
        }
        let mod_clone = module.clone();
        return Some(mod_clone.handle_request_impl(req));
    }
    return None;
}
