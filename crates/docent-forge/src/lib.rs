//! Forge accounts: which hosts the user has connected, and the token for each.

mod host;

pub use host::Host;

/// Why a host could not be connected.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConnectError {
    #[error("{0:?} is not a host name. Enter one like github.com or github.example.com.")]
    InvalidHost(String),
}
