/* arrays_pointers.c
 * Array and pointer interoperability; pass array to function that modifies it.
 */

void set_first(int *a, int v) {
    a[0] = v;
}

int main() {
    int nums[3];
    nums[0] = 5;
    nums[1] = 6;
    nums[2] = 7;

    set_first(nums, 99);

    return nums[0] + nums[2]; /* expect 99 + 7 = 106 */
}
