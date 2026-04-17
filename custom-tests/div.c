/* div.c
 * Division test.
 * x = 10 / 3 = 3
 * y = 7 / 2 = 3
 * putchar(x + 48) outputs '3' (ASCII 51)
 * putchar(y + 48) outputs '3' (ASCII 51)
 * putchar(10) outputs newline
 * return x / y = 3 / 3 = 1
 * Expected output: "33" + newline
 */

void putchar(char c){
    __asm(
        "Load %c",
        "Output"
    );
}

int main(void){
    int x = 10 / 3;
    int y = 7 / 2;
    putchar(x + 48);
    putchar(y + 48);
    putchar(10);
    return x / y;
}