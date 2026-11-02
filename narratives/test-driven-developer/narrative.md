i practice test-driven development (tdd)
i want to build my business logic into a library and have an automated test suite runner executable
i should be able to run `glue run unit_tests` directly

write include/calculator.h:
```c
#ifndef CALCULATOR_H
#define CALCULATOR_H

int add(int a, int b);
int subtract(int a, int b);

#endif
```

write src/calculator.c:
```c
#include "calculator.h"

int add(int a, int b) { return a + b; }
int subtract(int a, int b) { return a - b; }
```

write tests/test_runner.c:
```c
#include <stdio.h>
#include <assert.h>
#include "calculator.h"

int main(void) {
    assert(add(10, 5) == 15);
    assert(subtract(10, 5) == 5);
    puts("ALL ASSERTIONS PASSED! [100% OK]");
    return 0;
}
```

write here.glue:
```glue
namespace math;

static_lib.add(calculator)
    .sources("src/calculator.c")
    .include.public("include")

exec.add(unit_tests)
    .sources("tests/test_runner.c")
    .depends(math::calculator)
```

run:
glue run unit_tests

and all tests pass with zero friction!
