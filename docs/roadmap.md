# MARIE-C Compiler Roadmap

## Project Goal
Compile a subset of C into Marie assembly (`.mas` files).

## Current Status
The compiler already supports a substantial C subset including:
- Types: `int`, `char`, `void`, pointers, fixed-size arrays, structs, enums
- Declarations: globals, locals, functions, typedefs
- Statements: compound, expression, if/else, for, while, return, inline asm
- Expressions: identifiers, integers, unary ops, binary ops, assignment, function calls, array indexing, member access

## Excluded Features
- Bitwise operators (`&`, `|`, `^`, `<<`, `>>`)
- `goto` and labels
- Preprocessor (input assumed preprocessed)
- `static` variables (forbidden by Marie target)
- Division semantics (operations exist but division by zero not checked)

---

## Roadmap: Planned Features

### Phase 1: Simple Additions

#### Increment/Decrement (`++`, `--`)
- Postfix: `i++`, `i--`
- Prefix: `++i`, `--i`
- AST: Add `Increment` and `Decrement` variants to `Expression` enum
- Semantic: Validate lvalue, check for undefined variables
- Codegen: Convert to add/subtract with 1

#### Ternary Conditional (`?:`)
- Syntax: `cond ? expr1 : expr2`
- AST: Add `Ternary` variant to `Expression` enum
- Codegen: Lower using br/breq to conditional jump

#### Comma Operator in Expressions
- Support: `expr1, expr2` in parenthesized contexts
- Codegen: Evaluate left-to-right, keep only final value

### Phase 2: Medium Complexity

#### `sizeof` Operator
- Syntax: `sizeof(type)` and `sizeof(expr)`
- AST: Add `Sizeof` variant to `Expression` enum
- Semantic: Constexpr evaluation where possible
- Codegen: Constant folding for known types

#### `do-while` Loop
- Syntax: `do stmt while (expr);`
- AST: Add `DoWhile` variant to `Statement` enum
- Codegen: Loop with post-condition check

#### `continue` Statement
- Syntax: `continue;`
- Semantic: Must be inside iteration statement
- Codegen: Jump to loop start/end of condition

### Phase 3: More Complex

#### `switch`/`case`/`break`
- Syntax: `switch (expr) { case const: stmt... [default: stmt...] }`
- AST: Add `Switch` and `SwitchCase` variants
- Semantic: Case values must be constant integers, no duplicates
- Codegen: Jump table or if-chain lowering

#### `typedef` Semantics
- Status: Implemented for global typedefs
- Local typedefs inside function bodies are rejected
- Already has typedef table, type resolution, circular reference detection

#### Function Prototypes
- Syntax: `int foo(int a, int b);` (without body)
- Semantic: Validate definitions against declared prototypes
- Allow forward declarations

### Phase 4: Future/Advanced

#### Variadic Functions
- Syntax: `int foo(int a, ...);`
- Requires: `<stdarg.h>` handling, `va_start`/`va_arg`/`va_end`

#### Function Pointers
- Syntax: `int (*fp)(int);`
- Requires: Function type representation and call through pointer

---

## Non-Goals (Not Planned)

- Bitwise operators and shifts
- `goto` and labels
- Preprocessor (input assumed preprocessed)
- Full C standard library
- Optimizations beyond basic constant folding