i am a developer with many source files in subdirectories
i want to use glob patterns like `src/*.c` for simple batches
and regular expression patterns like `regex:algo/solver_v[0-9]+\.c` for versioned files
while safely excluding draft or experimental files like `algo/solver_draft.c`

write include/core.h:
```c
#ifndef CORE_H
#define CORE_H

int core_add(int a, int b);
int core_mul(int a, int b);

#endif
```

write src/add.c:
```c
#include "core.h"

int core_add(int a, int b) { return a + b; }
```

write src/mul.c:
```c
#include "core.h"

int core_mul(int a, int b) { return a * b; }
```

write include/solver.h:
```c
#ifndef SOLVER_H
#define SOLVER_H

int solver_v1_compute(int x);
int solver_v2_compute(int x);

#endif
```

write algo/solver_v1.c:
```c
#include "solver.h"

int solver_v1_compute(int x) { return x * 10; }
```

write algo/solver_v2.c:
```c
#include "solver.h"

int solver_v2_compute(int x) { return x * 100; }
```

write algo/solver_draft.c:
```c
// Ignored by regex:algo/solver_v[0-9]+\.c
#error "Draft file should not be compiled"
```

write app/main.c:
```c
#include <stdio.h>
#include "core.h"
#include "solver.h"

int main(void) {
    printf("Add: %d, Mul: %d\n", core_add(3, 4), core_mul(3, 4));
    printf("Solver V1: %d, Solver V2: %d\n", solver_v1_compute(5), solver_v2_compute(5));
    return 0;
}
```

write here.glue:
```glue
namespace math;

static_lib.add(core)
    .sources("src/*.c")
    .include.public("include")

static_lib.add(solver)
    .sources("regex:algo/solver_v[0-9]+\\.c")
    .include.public("include")

exec.add(app)
    .sources("app/*.c")
    .depends(math::core, math::solver)
```

run:
glue run app

and glue compiles the glob sources, the regex-filtered sources, ignores the draft file, and outputs:
Add: 7, Mul: 12
Solver V1: 50, Solver V2: 500
