// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Storage layer for Smelt

mod sqlite;

#[cfg(test)]
mod tests;

pub use sqlite::SqliteStorage;
