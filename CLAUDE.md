# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands
- Build: `cargo build --release`
- Check: `cargo check`
- Run: `cargo run`
- Test: `cargo test`
- Single test: `cargo test <test_name>`
- Lint: `cargo clippy --all-targets --all-features -- -W clippy::pedantic -D warnings`
- Format: `cargo fmt`

## Code Style Guidelines
- Use 4-space indentation
- Imports organized in three groups: standard library, external crates, internal modules
- Use snake_case for variables/functions, PascalCase for types/traits, SCREAMING_SNAKE_CASE for constants
- Custom error types with proper std::error::Error and Display implementations
- Errors propagated with `?` operator, using anyhow::Result for public APIs
- Document public APIs with `///` doc comments, especially error conditions
- Functions should be small, focused, and descriptive
- Implement standard traits (Debug, Display, etc.) when appropriate
- Terminal UI uses ratatui and crossterm
- Follow Rust's standard error handling patterns with detailed context
- Use block-style where clauses for complex generic constraints

When developing, ensure all clippy warnings are fixed, as warnings are treated as errors.