// Test: For loop with postfix increment prints "12345"
// Expected output: 12345
// Uses postfix increment in for loop - x++ returns old value for printing, then increments

void putchar(int c) {
    __asm("Load %c", "Output");
}

// Print numbers 1 through 5 using postfix increment
int main(void) {
    int count = 0;
    
    int i;
    for (i = 1; i <= 5; i++) {
        putchar(i + 48); // Convert to ASCII digit
        count++;
    }
    
    return count; // Should return 5
}
