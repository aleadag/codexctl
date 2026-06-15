pub mod cli;
pub mod config;
pub mod policy;
pub mod prompt;
pub mod publish;
pub mod sources;
pub mod store;
pub mod submit;
pub mod worktree;

pub type LoopResult<T> = Result<T, String>;
