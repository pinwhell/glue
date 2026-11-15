#include "fastmath.h"

FASTMATH_API int fastmath_multiply(int a, int b) {
    return a * b;
}

FASTMATH_API int fastmath_power(int base, int exp) {
    int result = 1;
    for (int i = 0; i < exp; ++i) {
        result *= base;
    }
    return result;
}
