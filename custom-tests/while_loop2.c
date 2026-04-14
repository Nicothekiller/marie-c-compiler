// Test 2: While loop with nested if and multiple variables
// Computes factorial of 5 (120)
// Expected output: 120
int main(void) {
    int n;
    int result;
    n = 5;
    result = 1;
    while (n > 0) {
        if (result < 1000) {
            result = result * n;
        }
        n = n - 1;
    }
    return result;
}