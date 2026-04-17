# MARIE Instruction Set

Each instruction is 16 bits: first 4 bits = opcode, last 12 bits = address.

## Code Table

| Type | Instruction | Hex Opcode | Summary |
|------|------------|------------|---------|
| **Arithmetic** | | | |
| | Add X | 3 | AC ← AC + M[X] |
| | Subt X | 4 | AC ← AC - M[X] |
| | AddI X | B | Add Indirect: use value at X as address |
| **Data Transfer** | | | |
| | Load X | 1 | Load M[X] into AC |
| | LoadImmi X | A | Load 12-bit unsigned value X into AC |
| | Clear | - | LoadImmi 0 (AC ← 0) |
| | Store X | 2 | Store AC into M[X] |
| **I/O** | | | |
| | Input | 5 | Request user input, store in AC |
| | Output | 6 | Print value from AC |
| **Branch** | | | |
| | Jump X | 9 | Jump to address X |
| | Skipcond C | 8 | Skip next instruction based on AC: |
| | | 000 | Skip if AC < 0 |
| | | 400 | Skip if AC = 0 |
| | | 800 | Skip if AC > 0 |
| | | 0C00 | Skip if AC ≠ 0 |
| **Subroutine** | | | |
| | JnS X | 0 | Store PC at X, jump to X+1 |
| | JumpI X | C | Jump to address in M[X] |
| **Indirect Addressing** | | | |
| | LoadI | D | Load from indirect address |
| | StoreI | E | Store to indirect address |
| **Halt** | | | |
| | Halt | 7 | End program |

## Usage in Compiler

### Arithmetic Helpers

Multiplication and division require helper routines since MARIE lacks native support.

**Multiplication** - iterative addition:
```
helper_mul_body, Clear
Store helper_mul_acc
helper_mul_loop, Load helper_mul_rhs
Skipcond 400     ; if AC == 0, exit
Jump helper_mul_done
helper_mul_continue, Load helper_mul_acc
Add helper_mul_lhs
Store helper_mul_acc
Load helper_mul_rhs
Subt const_one
Store helper_mul_rhs
Jump helper_mul_loop
```

**Division** - iterative subtraction:
```
helper_div_body, Clear
Store helper_div_quotient
helper_div_loop, Load helper_div_rhs
Skipcond 400     ; if divisor == 0, error (not handled)
Jump helper_div_done
; Compare: if dividend < divisor, done
Load helper_div_dividend
Subt helper_div_rhs
Skipcond 400     ; if result == 0, exact division
Jump helper_div_continue
Skipcond 800     ; if result < 0, done
Jump helper_div_done
helper_div_continue, Load helper_div_dividend
Subt helper_div_rhs
Store helper_div_dividend
Load helper_div_quotient
Add const_one
Store helper_div_quotient
Jump helper_div_loop
helper_div_done, Load helper_div_quotient
```

### Comparison Patterns

**AC < 0**: `Skipcond 000`
**AC == 0**: `Skipcond 400`
**AC > 0**: `Skipcond 800`
**AC != 0**: `Skipcond 0C00`