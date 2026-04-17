// Test: Prefix increment prints "BC" (increment returns new value)
// Expected output: BC
// ++x returns the incremented value, so x becomes 66='B', print 'B', then x becomes 67='C', print 'C'

void putchar(int c) {
    __asm("Load %c", "Output");
}

int main(void) {
    int x = 65; // 'A'
    putchar(++x); // x becomes 66, prints 'B'
    putchar(++x); // x becomes 67, prints 'C'
    return 0;
}