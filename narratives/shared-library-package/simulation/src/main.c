#include <stdio.h>
#include "fastmath.h"

int main(void) {
    int mul = fastmath_multiply(6, 7);
    int pow = fastmath_power(2, 8);
    printf("FastMath Multiply: 6 * 7 = %d\n", mul);
    printf("FastMath Power: 2 ^ 8 = %d\n", pow);
    return 0;
}
