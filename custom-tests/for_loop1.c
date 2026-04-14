// Test 1: For loop - sums 0+1+2+...+9 = 45
// Expected output: 45
int main(void) {
    int i;
    int sum;
    sum = 0;
    for (i = 0; i < 10; i = i + 1) {
        sum = sum + i;
    }
    return sum;
}