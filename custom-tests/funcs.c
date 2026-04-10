/* funcs.c
 * Tests simple function calls, parameters and return.
 */

int add(int x, int y) {
    return x + y;
}

int times2(int v) {
    return v * 2;
}

int main() {
    int res;
    res = add(5, 7);
    res = times2(res);
    return res; /* expect (5+7)*2 = 24 */
}
