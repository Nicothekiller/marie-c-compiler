# Parser Plan

## Goal
Define and implement a practical v0 C subset grammar in Pest for `marie-c-compiler`.

This subset is intentionally small so we can move quickly from parse-only to AST and codegen.

## Input assumptions
- Input has already gone through the C preprocessor.
- We parse plain C source text after preprocessing.

## v0 language scope

### Supported types
- `int`
- `char`
- `void`

### Supported declarations
- Global variable declarations
- Local variable declarations
- Function definitions
- Function prototypes
- Pointer declarators (e.g. `int *p;`)
- Fixed-size array declarators (e.g. `int a[10];`)

### Supported statements
- Compound/block statement `{ ... }`
- Expression statement (`expr;` and empty `;`)
- Selection statement:
  - `if (...) stmt`
  - `if (...) stmt else stmt`
  - `else if` chains via nested `if`
- Jump statement:
  - `return;`
  - `return expr;`

### Supported expressions
- Primary: identifiers, integer constants, parenthesized expressions
- Postfix:
  - function call `f(...)`
  - array indexing `a[i]`
- Unary:
  - `&`, `*`, unary `+`, unary `-`, logical `!`
- Binary arithmetic:
  - `*`, `%`, `+`, `-`
- Relational/equality:
  - `<`, `<=`, `>`, `>=`, `==`, `!=`
- Logical:
  - `&&`, `||`
- Assignment:
  - `=`

## Explicitly out of scope (for v0)
- Division `/`
- Bitwise operators `&` (binary), `|`, `^`
- Bit shifts `<<`, `>>`
- Ternary `?:`
- Increment/decrement `++`, `--`
- `for`, `while`, `do`, `switch`, `case`, `break`, `continue`
- `goto` and labels
- `static` storage class
- `typedef`, `enum`, `union`
- Initializer lists (`{...}`) for arrays/structs
- String literals and character literals (for now)
- Cast expressions `(type)expr` (for now)
- Inline assembly extension (planned for v2)

## Grammar design notes
- Keep classic C precedence layering:
  - primary -> postfix -> unary -> multiplicative -> additive -> relational -> equality -> logical_and -> logical_or -> assignment
- Use declaration-specifier + declarator shape for C-like declarations.
- Keep parser permissive where needed; semantic rejections and diagnostics come in v1.
- Parse `else if` naturally via `else` + nested `if` statement.

## v1 specification

### Goals
- Keep v0 syntax stable and add robust diagnostics + semantic validation.
- Expand grammar only where needed for explicit rejection of unsupported C features.

### Parser/grammar scope
- Preserve all v0 constructs.
- Add `while (expr) statement` as a supported v1 iteration statement.
- Recognize unsupported operators/tokens so we can emit precise errors:
  - `/`, `<<`, `>>`, binary `&`, `|`, `^`, compound assignments for unsupported ops.
- Recognize `static` and reject it explicitly with a dedicated diagnostic.
- Keep `goto`/labels and inline asm out of v1 grammar implementation.

### Semantic validation scope
- Declaration and symbol checks:
  - duplicate global names,
  - duplicate local names in the same scope,
  - unknown identifiers,
  - function declaration/definition signature mismatches.
- Type checks for v0/v1 subset:
  - assignment compatibility between integer-like and pointer-like values,
  - pointer dereference/address-of legality,
  - array indexing constraints,
  - return type consistency.
- Expression checks:
  - lvalue requirement for assignment targets,
  - restricted pointer arithmetic rules for supported operators.

### Diagnostics scope
- Provide stable, actionable diagnostics with source locations.
- Start standardizing error categories:
  - parse error,
  - unsupported feature,
  - type error,
  - name resolution error.

### Non-goals for v1
- Full C type system.
- Optimizations.
- Full control-flow feature parity with C (only `while` is added beyond v0 control flow).

## v2 specification

### Goals
- Add power features needed for low-level control in Marie targets.
- Introduce compiler extension support while keeping the core subset predictable.

### Inline assembly extension (planned)
- Add statement-level inline asm first:
  - `__asm("MARIE INSTRUCTIONS");`
- Parse asm blocks as dedicated AST node, e.g. `Stmt::InlineAsm`.
- Backend emits asm payload directly at statement position.
- Future optional extension points:
  - `volatile` marker,
  - operand bindings/constraints (deferred unless needed).

### Control-flow extensions
- Optionally add labels + `goto`:
  - labeled statement syntax,
  - `goto <label>;` jump statement,
  - diagnostics for unknown/duplicate labels.
- Keep this optional if project policy decides to omit `goto` permanently.

### Data/model extensions
- Expand `struct` and array handling beyond v0 baseline as needed.
- Improve pointer semantics and memory layout rules for backend lowering.

### Backend alignment goals
- Ensure all v2 constructs map cleanly to Marie labels/jumps/data layout.
- Keep generated `.mas` output deterministic and easy to inspect.
