void putchar(char c){
    __asm(
        "Load %c",
        "Output"
    );
}

typedef struct Point {
    int x;
    int y;
} Point;

int main(void){
    Point p;
    p.x = 1;
    p.y = 2;

    putchar(p.x + 48);
    putchar(p.y + 48);
    putchar(10);

    return 0;
}
