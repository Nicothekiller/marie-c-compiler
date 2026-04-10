/* ptr_arith.c
 * Pointer arithmetic and indexing.
 */

int main() {
    int arr[4];
    int *p;
    int i;

    arr[0] = 1;
    arr[1] = 2;
    arr[2] = 3;
    arr[3] = 4;

    p = arr; /* p points to arr[0] */
    i = *(p + 2); /* expect arr[2] == 3 */

    /* modify through pointer arithmetic */
    *(p + 1) = 20; /* arr[1] = 20 */

    return arr[1] + i; /* expect 20 + 3 = 23 */
}
