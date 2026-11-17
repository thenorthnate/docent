//! Docent's local git operations. Every `git` process Docent starts goes
//! through [`Git`], so its environment is scrubbed in one place.

mod clone;
mod error;
mod remote;
mod runner;
mod worktree;

pub use clone::BoundClone;
pub use error::GitError;
pub use runner::Git;
pub use worktree::PrWorktree;
