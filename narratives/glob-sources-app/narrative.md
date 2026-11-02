i am a developer with many source files in subdirectories
i do not want to list each file individually in my build definition
i want to write `src/*.c` and have glue discover and compile all matching files deterministically

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

write app/main.c:
```c
#include <stdio.h>
#include "core.h"

int main(void) {
    printf("Add: %d, Mul: %d\n", core_add(3, 4), core_mul(3, 4));
    return 0;
}
```

write here.glue:
```glue
namespace math;

static_lib.add(core)
    .sources("src/*.c")
    .include.public("include")

exec.add(app)
    .sources("app/*.c")
    .depends(math::core)
```

run:
glue run app

and glue expands "src/*.c" to all matching c sources and executes:
Add: 7, Mul: 12
