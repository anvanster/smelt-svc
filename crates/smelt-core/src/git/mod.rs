//! Git integration for Smelt

mod git2_impl;
mod interface;

#[cfg(test)]
mod tests;

pub use git2_impl::Git2Interface;
pub use interface::{CommitInfo, GitInterface};
