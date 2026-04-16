int putchar(int c) { __asm("Load %c"); __asm("Output"); return c; }

int main(void) {
    char msg[4] = "ABC";
    int i = 0;
    while (i < 4) {
        putchar(msg[i]);
        i = i + 1;
    }
    putchar(10);
    return 0;
}
