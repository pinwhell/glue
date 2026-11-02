#include <stdio.h>
#include <assert.h>
#include "calculator.h"

int main(void) {
    assert(add(10, 5) == 15);
    assert(subtract(10, 5) == 5);
    puts("ALL ASSERTIONS PASSED! [100% OK]");
    return 0;
}
