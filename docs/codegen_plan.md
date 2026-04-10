# Marie Codegen Plan (v0)

## Goal
Implement a deterministic backend that lowers the semantically validated AST into MARIE `.mas` assembly.

This plan assumes the current compiler pipeline:
- Parse -> AST
- Semantic analysis
- Codegen backend (`Codegen` trait)

## Constraints and Design Decisions

### Target constraints
- MARIE has no native runtime stack in this project model.
- Recursion is forbidden.
- Variables are lowered to static memory cells.
- Output uses symbolic labels for readability and patchability.

### Program layout (high level)
1. Entry/main code appears at top of emitted file.
2. Main-related static data and globals are emitted in data section.
3. Function code blocks follow with static parameter/return cells.
4. Helper routines (e.g. multiply/mod) are included as needed.

### Function ABI model (no stack)
For each function `f` we reserve static labels:
- `f_param_<n>` for each parameter
- `f` as JnS entry storage cell (per MARIE convention)
- function body starts at `f_body`
- `f_ret` for return value
- epilogue returns with `JumpI f`

Call flow (conceptual):
1. Evaluate argument expressions in caller.
2. Store values into callee `f_param_<n>` cells.
3. `JnS f`
4. Read `f_ret` for expression result.

Recursion ban means this ABI is valid only when calls are non-reentrant.

## Label Strategy

### Why symbolic labels
Inlining raw numeric addresses everywhere is brittle and hard to debug.
We keep symbolic labels and let assembler resolve addresses.

### Internal labels
Compiler emits unique internal labels for all symbols/temporaries:
- globals: `g_<name>`
- functions: `fn_<name>` style cells/sections
- locals: `v_<fn>_<scope>_<name>`
- temps: `tmp_<fn>_<n>`
- control-flow: `if_<n>_then/else/end`, `loop_<n>_*`

### Inline asm compatibility
Inline asm (v2) should reference source symbols through placeholders (planned):
- `%name` for in-scope source name
- compiler resolves placeholders to internal labels pre-emit

This preserves readability and avoids exposing unstable mangled internals.

## Emission Model

### Core abstraction
Backend keeps an emitter state:
- instruction list
- data declarations list
- helper inclusion flags
- label counter
- function/local symbol-to-label map

### Two-phase lowering
1. **Planning/allocation phase**
   - Allocate labels for globals, functions, locals, params, temps.
2. **Emission phase**
   - Emit instructions/statements/expressions using allocated labels.

## Expression Lowering Plan

All expression lowering returns result in AC, with temp spills as needed.

Order of implementation:
1. Literals and identifiers
2. Assignment
3. `+` and `-`
4. `*` and `%` via helper routines
5. Comparisons (`== != < <= > >=`) normalized to `0/1`
6. Logical `&& || !` normalized to `0/1`
7. Calls and indexing

## Statement Lowering Plan

1. Expression statement
2. Return statement
3. Block sequencing
4. If / else-if / else using generated branch labels

Condition branch pattern should use `Skipcond` + `Jump` sequences consistently.

## MARIE Instruction Usage (v0)

Primary instructions expected in generated code:
- `Load`, `Store`
- `Add`, `Subt`
- `LoadImmi` / `Clear`
- `Skipcond`
- `Jump`
- `JnS`, `JumpI`
- `Halt`

Optional indirect forms (`LoadI`, `StoreI`) used when indexing/pointer lowering needs them.

## Helpers

### Planned helpers
- `mul_helper` (if multiplication not emitted inline)
- `mod_helper`
- comparison helpers if direct pattern emission becomes too verbose

Helpers are emitted only when referenced.

## Determinism Rules
- Stable output ordering for globals/functions/helpers.
- Stable label generation order based on source traversal.
- No randomness or hash-dependent naming.

## Testing Strategy

### Unit tests (backend)
- AST snippet -> emitted assembly contains expected instruction/label patterns.

### Integration tests
- End-to-end compile tests from source to `.mas` output snapshots.

### Safety checks
- Ensure no unresolved label placeholders remain before final output.
- Ensure recursion call graph is rejected (or flagged) in semantic/codegen guard.

## Implementation Phases

### Phase A (minimal runnable)
- Main function only
- Globals + integer locals
- `return`, assignment, `+`, `-`, `if`

### Phase B
- Full function call ABI (non-recursive)
- `*`, `%`, comparisons, logical ops

### Phase C
- Arrays/pointers in codegen path
- helper cleanup and output polish

### Phase D (future v2 prep)
- inline asm placeholder resolution pipeline

## Non-Goals (v0 backend)
- Recursion support
- dynamic stack frame simulation
- optimization passes
- full C ABI compatibility
