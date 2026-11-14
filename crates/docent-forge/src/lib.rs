//! Forge accounts: which hosts the user has connected, the token for each,
//! and what Docent asks of the forge with it.

mod accounts;
mod error;
#[cfg(feature = "fake")]
mod fake;
mod forge;
mod gh;
pub mod github;
mod host;
mod repo;
mod token;

pub use accounts::{AccountSummary, Accounts};
pub use error::ConnectError;
#[cfg(feature = "fake")]
pub use fake::FakeForge;
pub use forge::{Forge, HostFailure, PullSummary, Section, SectionPulls};
pub use gh::GhCli;
pub use host::Host;
pub use repo::{ForgeRepo, RepoError};
pub use token::Token;
