// Test: Postfix increment prints "AB" (postfix returns old value)
// Expected output: AB
// x++ returns the old value, then increments
// x=65 ('A'), x++ returns 65='A', x becomes 66
// then x++ returns 66='B', x becomes 67

void putchar(int c) {
    __asm("Load %c", "Output");
}

int main(void) {
    int x = 65; // 'A'
    putchar(x++); // returns 65='A', x becomes 66
    putchar(x++); // returns 66='B', x becomes 67
    return 0;
}