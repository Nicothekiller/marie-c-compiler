Tests for marie-c-compiler

Place these `.c` files through your compiler to generate `.mas` outputs and run them with the Marie simulator. Each test includes a brief description and expected behavior.

Files:
- `simple.c` - prints a constant (via `main` return) or stores a value in memory (depending on your runtime). Expected: basic compile.
- `arith.c` - integer arithmetic and return value (division not used — compiler does not support division).
- `funcs.c` - function calls, parameters, return values.
- `ptr_basic.c` - basic pointer read/write, pointer to int.
- `ptr_arith.c` - pointer arithmetic and indexing.
- `arrays_pointers.c` - array semantics with pointers and function that modifies array elements.

Notes:
- These tests use only a small subset of C (ints, pointers, functions, arrays). Adjust as needed for your compiler's runtime and ABI.
- If your compiler expects `printf` or I/O it may not be supported; these tests use return values or memory stores instead.

Usage:
- Compile: `cargo run -- path/to/tests/<file>.c` (or your CLI invocation).
- Run resulting `.mas` in your Marie simulator.

Expected behaviors (high-level):
- Programs should assemble without parser errors.
- Pointer tests cover load/store via pointers and pointer arithmetic.

If you want I can adapt these to your exact runtime ABI (how `main` returns values or how to print). Let me know how `main` should communicate results to the simulator (return register/memory address).