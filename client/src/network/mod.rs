pub mod messages;
pub(crate) mod message_handler;
pub(crate) mod protocol;

#[cfg(not(target_arch = "wasm32"))]
mod client;

#[cfg(target_arch = "wasm32")]
mod wasm_client;

#[cfg(not(target_arch = "wasm32"))]
pub use client::NetworkClient;
#[cfg(target_arch = "wasm32")]
pub use wasm_client::NetworkClient;
