# AGENTS.md

This file provides instructions for coding agents working in this repository.
Scope: entire repository unless overridden by nested `AGENTS.md` files.

## Project Overview
- Project: `marie-c-compiler`
- Language: Rust
- Goal: compile a subset of C into Marie assembly (`.mas` files)
- Parser technology: `pest`
- Input assumption: C source has already been preprocessed by the user

## Current Architecture
- `src/lib.rs`: library crate entrypoint
- `src/main.rs`: CLI frontend (`clap`-based)
- `src/parser.rs`: parser frontend using Pest grammar
- `src/parser/grammar.pest`: grammar source
- `src/ast.rs`: AST definitions
- `src/compiler.rs`: pipeline coordinator
- `src/codegen.rs`: Marie output emitter
- `src/error.rs`: shared compiler error types
- `docs/parser_plan.md`: parser v0/v1/v2 roadmap

## High-Level Pipeline
- Parse source text into parse tree
- Lower parse tree into AST
- Compile AST into Marie assembly text
- Write output with `.mas` extension

## File and Module Conventions
- Prefer modern module layout:
  - `foo.rs`
  - `foo/bar.rs`
- Avoid creating `foo/mod.rs` unless explicitly requested
- Keep `lib.rs` as primary public module index

## Coding Style
- Keep changes minimal and focused
- Prefer clear names over abbreviations
- Avoid one-letter variable names
- Do not add inline comments unless requested
- Do not add license/copyright headers
- Keep APIs small and composable

## Rust Guidelines
- Prefer explicit enums/structs for AST nodes
- Derive traits only when needed (`Debug`, `Clone`, etc.)
- Keep error conversion straightforward (`From` impls where useful)
- Avoid unnecessary generics in early-stage compiler modules
- Keep ownership/borrowing simple and readable

## Parser Guidelines (Pest)
- Grammar should reflect reduced C subset, not full ANSI C
- Keep operator precedence explicit in grammar rules
- Reserve unsupported features for v1 diagnostics
- Prefer deterministic grammar over overly clever rules
- Keep keyword matching safe against identifier collisions
- Maintain whitespace and comment handling rules

## AST Guidelines
- Model language features with explicit node variants
- Keep v0 AST aligned with `docs/parser_plan.md`
- Separate declarations, statements, and expressions cleanly
- Encode operators as enums instead of raw strings
- Preserve room for v1/v2 expansion without breaking names

## Codegen Guidelines
- Output target extension: `.mas`
- Prefer deterministic output ordering
- Keep placeholder behavior simple until lowering is complete
- Keep backend interfaces independent from CLI concerns

## Unsupported/Deferred Features
- `static` variables are forbidden by target constraints
- Division `/` is excluded from current subset
- Bitwise/shift operators are excluded for now
- `goto` is deferred and may remain optional
- Inline asm is planned as a v2 statement-level extension

## CLI Guidelines
- Use `clap` for argument parsing
- Keep CLI behavior predictable and explicit
- Default output path should derive from input with `.mas`
- Prefer stable user-facing messages

## Testing Expectations
- Add/adjust tests for behavior changes
- Favor small unit tests near changed modules
- Keep one integration test for end-to-end CLI behavior
- Run `cargo test` after meaningful code changes
- Use `cargo check` for quick validation during iteration

## Documentation Expectations
- Keep docs concise while project evolves
- Update `docs/parser_plan.md` when scope changes
- Add function-level docs for new public functions
- Avoid large speculative docs until features are stable

## Git and Change Discipline
- Do not create commits unless explicitly requested
- Do not create branches unless explicitly requested
- Avoid touching unrelated files
- Keep diffs review-friendly and scoped

## When in Doubt
- Follow user instructions first
- Then follow nested `AGENTS.md` if present
- Then follow this file
- Prefer simpler implementation that unblocks progress
- Ask for clarification when feature scope is ambiguous

## Quick Validation Checklist
- Project compiles (`cargo check`)
- Tests pass (`cargo test`)
- Grammar changes don’t break parser smoke tests
- AST changes preserve existing tests
- CLI still emits `.mas` output by default
