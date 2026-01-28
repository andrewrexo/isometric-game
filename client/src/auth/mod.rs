pub mod types;

#[cfg(not(target_arch = "wasm32"))]
mod client;

#[cfg(target_arch = "wasm32")]
mod wasm_client;

#[cfg(not(target_arch = "wasm32"))]
pub use client::AuthClient;

#[cfg(target_arch = "wasm32")]
pub use wasm_client::{AuthClient, AuthResult};

pub use types::{AuthError, AuthSession, CharacterInfo};
