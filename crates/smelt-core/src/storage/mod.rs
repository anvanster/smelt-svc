//! Storage layer for Smelt

mod sqlite;

#[cfg(test)]
mod tests;

pub use sqlite::SqliteStorage;
