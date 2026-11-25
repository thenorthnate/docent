//! Docent's local git operations. Every `git` process Docent starts goes
//! through [`Git`], so its environment is scrubbed in one place.

mod changed_files;
mod clone;
mod clone_repo;
mod error;
mod remote;
mod runner;
mod worktree;
mod worktrees_root;

pub use changed_files::{ChangedFile, FileStatus, LineCounts};
pub use clone::BoundClone;
pub use clone_repo::CloneProgress;
pub use error::GitError;
pub use runner::Git;
pub use worktree::PrWorktree;
pub use worktrees_root::{PrWorktreePath, WorktreesRoot};
