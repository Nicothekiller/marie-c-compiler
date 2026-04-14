# marie-c-compiler

A small compiler that lowers a deliberately limited subset of C to [MARIE](https://marie.js.org/) assembly (.mas).
The compiler currently doesnt have a preprocessor (aka #include), so make sure to feed the compiler
pre-processed C files.

Current features (v0.1.0)
- Function definitions, parameters (including the void marker), and returns
- Integer arithmetic (+, -, *, %), relational and logical operators
- Unary operators: +, -, logical not, address-of (&), dereference (*)
- Pointers and arrays: declarations, initialization, indexing, address-of decay
- Pointer arithmetic: pointer+int, int+pointer, pointer-int, pointer-pointer (same object)
- Codegen emits ADR address constants, per-element array storage, and helper routines for multiply/modulo

Quickstart
1. Build
   ```bash
   cargo build --release
   ```

2. Test
   ```bash
   cargo test
   ```

3. Run (assumes input is preprocessed C, e.g. foo.i)
   ```bash
   cargo run -- foo.i
   ```
   This produces foo.mas in the same directory.

Notes
- This project intentionally targets a small, well-specified subset of C. See docs/parser_plan.md for details on the language boundary.
- The compiler assumes inputs are preprocessed; it does not run the C preprocessor.

Contributing
- Run tests and add focused unit tests for parser, semantic, and codegen behavior.
