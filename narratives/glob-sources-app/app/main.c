#include <stdio.h>
#include "core.h"
#include "solver.h"

int main(void) {
    printf("Add: %d, Mul: %d\n", core_add(3, 4), core_mul(3, 4));
    printf("Solver V1: %d, Solver V2: %d\n", solver_v1_compute(5), solver_v2_compute(5));
    return 0;
}
