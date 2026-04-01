# Smelt

Semantic version control layered over Git. Rust workspace with 6 crates.

## Build & Test
```bash
./scripts/build.sh --check       # fmt + clippy + tests
cargo test --workspace            # tests only
cargo test -p smelt-core          # test single crate
cargo test -p integration         # integration tests
cargo build --release             # release build (LTO, slow)
```

## Architecture
Crate dependency order:
1. `smelt-core` — types, graph storage, git integration
2. `smelt-memory` — episodic memory with embeddings (depends on core)
3. `smelt-validator` — semantic + architectural validation (depends on core)
4. `smelt-cli` — CLI binary (depends on core, memory, validator)
5. `smelt-api` — REST API / axum (depends on core, memory, validator)
6. `smelt-mcp` — MCP server (depends on core, memory, validator)
7. `tests/integration` — end-to-end tests

## Self-Referential Warning
This project IS the smelt MCP server. When rebuilding, do NOT call smelt MCP tools simultaneously. Use tempera and stellarion MCP tools instead while working on this project.

## Version
Workspace version: `workspace.package.version` in root `Cargo.toml` (currently `0.2.0`). Internal crate deps must match. Publishing to crates.io: `./scripts/publish.sh` (dependency order with 30s delays).

Tagging `v*` triggers GitHub Actions: check, build (4 platforms), release, publish.
