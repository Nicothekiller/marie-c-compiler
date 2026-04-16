void putchar(int c) {
    __asm("Load %c", "Output");
}

void print(char *str){
    while (*str != 0) {
        putchar(*str);
        str = ( str + 1 );
    }
}

int main(void) {
    char msg[20] = "Hello, world!\n";
    print(msg);
    return 0;
}
