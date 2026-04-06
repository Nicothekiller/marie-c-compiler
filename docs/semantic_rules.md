# Semantic Rules (v0)

This document defines what the compiler currently allows and rejects at semantic-analysis time.

The intent is to keep behavior deterministic while the project is still in early stages.

## Scope
- Applies to semantic analysis of the v0 language subset.
- Parser-level acceptance does not automatically imply semantic validity.
- Input is assumed to be preprocessed C.

## Type System (v0)

### Allowed types
- `int`
- `char`
- `void`
- Pointers: `T*`
- Fixed-size arrays: `T[N]`
- Function types for declarations/calls

### Behavior notes
- `char` is treated as integer-like for arithmetic and codegen.
- Arrays may decay to pointers in expression contexts where needed.

## Symbol and Scope Rules

### Allowed
- Global variable declarations.
- Function definitions only (no prototypes).
- Function parameters.
- Local variables in block scope.
- Shadowing across nested scopes.

### Rejected
- Duplicate symbol in the same scope.
- Duplicate function definition.
- Function prototype syntax (not part of this subset).
- Use of undeclared identifier.

## Statement Rules

### Allowed
- Block statement `{ ... }`.
- Expression statement `expr;` and empty statement `;`.
- `if (...) stmt` and `if (...) stmt else stmt`.
- `return;` in `void` functions.
- `return expr;` in non-`void` functions.

### Rejected
- `return;` in non-`void` functions.
- `return expr;` in `void` functions.
- Unsupported statement families (until explicitly added):
  - `for`, `while` (planned v1), `do`, `switch`
  - `break`, `continue`
  - `goto`, labels

## Expression Rules

### Identifier
- Must resolve to a declared symbol.

### Assignment (`lhs = rhs`)
- `lhs` must be assignable (lvalue).
- `rhs` must be assignable to `lhs` type.
- Assignment expression type is the `lhs` type.

### Lvalues (v0)
- Allowed:
  - identifier
  - dereference (`*ptr`)
  - index expression (`arr[idx]`)
- Rejected:
  - literals
  - call results
  - non-assignable computed expressions

### Unary operators
- `&x`: `x` must be lvalue; result type is pointer.
- `*x`: operand must be pointer; result is pointee type.
- `+x`, `-x`: operand must be integer-like.
- `!x`: operand must be scalar-like (integer/pointer).

### Binary operators
- Arithmetic `+ - * %`:
  - Allowed on integer-like operands (`int`, `char`).
- Relational `< <= > >=`:
  - Allowed on integer-like operands in v0.
- Equality `== !=`:
  - Allowed on integer-like operands.
  - Allowed on compatible pointer operands.
- Logical `&& ||`:
  - Allowed on scalar-like operands.
  - Result type is `int`.

### Calls
- Callee must be a function symbol.
- Argument count must match parameters.
- Each argument must be assignable to parameter type.

### Indexing
- Base must be pointer or array.
- Index must be integer-like.
- Result type is element type.

## Assignability and Conversions (v0)

### Allowed
- Exact same type.
- `char -> int`.
- `int -> char` (lossy behavior allowed in v0).
- Null literal `0` to pointer.

### Rejected
- Pointer to unrelated pointer type (unless explicitly made compatible later).
- Pointer to nonzero integer conversion.
- Non-pointer to pointer conversion (except null literal).
- Function type as runtime value.

## Explicitly Unsupported Features (v0)
- Division operator `/`.
- Bitwise operators `&` (binary), `|`, `^`.
- Shift operators `<<`, `>>`.
- Ternary operator `?:`.
- Increment/decrement `++`, `--`.
- `static` storage class.

## Diagnostics Expectations
- Each semantic error should include:
  - error category,
  - concise message,
  - source location (when spans are available).
- Prefer stable, actionable wording.

## Version Notes
- `while` loops are planned for v1 semantic support.
- Inline asm extension is planned for v2.
