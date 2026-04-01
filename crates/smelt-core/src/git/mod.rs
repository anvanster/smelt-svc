// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Git integration for Smelt

mod git2_impl;
mod interface;

#[cfg(test)]
mod tests;

pub use git2_impl::Git2Interface;
pub use interface::{CommitInfo, GitInterface};
