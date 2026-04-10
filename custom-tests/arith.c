/* arith.c
 * Integer arithmetic test.
 */

int main() {
    int a;
    int b;
    a = 10;
    b = 3;
    a = ( a * b + (a - b) ) % 7;
    return a; /* expect 10*3 + (10-3) = 30 + 7 = 37, 37 % 7 == 2 */
}
