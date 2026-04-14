// Test 2: For loop with nested while and conditional break
// Finds first number where sum exceeds 100
// Expected output: 14 (1+2+...+14 = 105 > 100)
int main(void) {
    int i;
    int sum;
    sum = 0;
    for (i = 1; i < 100; i = i + 1) {
        sum = sum + i;
        if (sum > 100) {
            return i;
        }
    }
    return 0;
}