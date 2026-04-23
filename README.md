# marie-c-compiler

A compiler that lowers a deliberately limited subset of C to [MARIE](https://marie.js.org/) assembly (`.mas`).

## Prerequisites

You need some tools to compile the project, namely:

- **Rust toolchain**: https://rustup.rs/

## Build

```bash
cargo build --release
```

The compiled binary will be at `target/release/marie-c-compiler`.

## Run

```bash
cargo run --release -- <input.c>
```

Arguments:
- `<input.c>`: Preprocessed C source file (`.c` or `.i`)

Output: Generates `<input>.mas` in the same directory as the input file.

## Test

```bash
cargo test
```

## Current Features (v0.3.1)

### Types
- `int`, `char`, `void`
- Pointers and fixed-size arrays
- Structs and enums
- Typedefs
- String literals

### Declarations
- Global and local variables
- Function definitions with parameters and return values

### Statements
- Compound statements
- Expression statements
- `if`/`else` conditionals
- `for` and `while` loops
- `return` statements
- Inline assembly (MARIE asm inside C)
    - Please also note that the __asm syntax is different from real compilers like gcc or clang, mostly due to their implementation being very ugly and hard to understand. This means that your lsp (diagonstic / autocomplete) will probably break.

### Expressions
- Identifiers and integer literals
- Unary operators: `+`, `-`, `!` (logical not), `&` (address-of), `*` (dereference)
- Binary operators: `+`, `-`, `*`, `%`, `/` (arithmetic), relational (`<`, `<=`, `>`, `>=`), equality (`==`, `!=`), logical (`&&`, `||`)
- Assignment expressions
- Function calls
- Array indexing and pointer arithmetic (`ptr+int`, `int+ptr`, `ptr-ptr`)
- Prefix and postfix increment/decrement (`++`, `--`)
- Struct member access (`.` and `->`)

### Codegen
- Generates ADR address constants for global variables
- Per-element array storage
- Helper routines for multiply, modulo, and division operations

## Excluded Features

- Bitwise operators (`&`, `|`, `^`, `<<`, `>>`)
- `goto` and labels
- `static` variables
- Preprocessor - input must be preprocessed C (no `#include`, `#define`, etc)

## Currently missing features

- `do {...} while` loops
- Variadic functions
- Switch
- Ternary operator

## Quick Example

Input (`example.c`):
```c
int add(int a, int b) {
    return a + b;
}

int main(void) {
    int x = 5;
    int y = 3;
    return add(x, y);
}
```

Compile:
```bash
cargo run --release -- example.c
```

Output: `example.mas` (Marie assembly)

A more complicated example:
```c
void putchar(char c){
    __asm(
        "Load %c",
        "Output"
    );
}

typedef struct Point {
    int x;
    int y;
} Point;

int main(void){
    Point p;
    p.x = 1;
    p.y = 2;

    putchar(p.x + 48);
    putchar(p.y + 48);
    putchar(10);

    return 0;
}
```

After compilation, this will print "12\n" on the marie simulator. You can look at the custom-tests directory for more examples.

## Testing

Run tests in `custom-tests/` to verify compilation:
```bash
cargo run --release -- custom-tests/simple.c
```

Then run the resulting `.mas` file in the [Marie simulator](https://marie.js.org/). There is also an script included to compile all tests that you can run while on the custom-tests directory if you have [Nushell](https://www.nushell.sh/)

## Documentation

- `docs/roadmap.md` - Planned features and language boundary
- `docs/marie-instructions.md` - MARIE instruction reference
- `docs/semantic_rules.md` - Semantic analysis rules
- `docs/codegen_plan.md` - Code generation plan

## Contributing

Please report all bugs as an issue on the repository.

Run tests before submitting changes:
```bash
cargo test
```

Add focused unit tests for parser, semantic, and codegen behavior.
