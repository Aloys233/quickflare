//! Tunnel-provider abstraction.
//!
//! We want to make it easy to plug in **ngrok**, **Pinggy**, **Tailscale
//! Funnel** or anything else later. The contract is small on purpose:
//! a provider knows how to spawn a child process for a given local port,
//! and how to recognise its own public URL inside the stdout stream.

pub mod cloudflared;
pub mod traits;

pub use traits::ProviderRegistry;
