/* enum.c
 * Enum variable + constant arithmetic test.
 * RED=0, GREEN=3, BLUE=4
 * c = GREEN = 3
 * return c + BLUE = 3 + 4 = 7
 */

enum Color { RED, GREEN = 3, BLUE };

int main(void){
    enum Color c;
    c = GREEN;
    return c + BLUE;
}