// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Crucible integration for architectural validation
//!
//! This module bridges Crucible's architectural validation with SmeltValidator.

mod adapter;

pub use adapter::CrucibleAdapter;
