#[cfg(any(test, target_arch = "wasm32"))]
mod extract_peer_arrays;

#[cfg(any(test, target_arch = "wasm32"))]
mod arena_url;

#[cfg(any(test, target_arch = "wasm32"))]
mod operator_parse;

#[cfg(target_arch = "wasm32")]
include!("wasm.rs");

#[cfg(not(target_arch = "wasm32"))]
pub fn host_build_placeholder() {}
