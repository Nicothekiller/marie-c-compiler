/* ptr_basic.c
 * Basic pointer read/write test.
 * Allocates an int, writes a value via pointer, reads it back.
 */

int main() {
    int a;
    int *p;
    a = 0;
    p = &a;
    *p = 123;
    return a; /* expect 123 */
}
