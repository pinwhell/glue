i am a cli tool developer
i want to build a command line application split into a core math engine library and a main cli executable
i do not want to configure makefiles or cmake scripts

write src/engine.c:
```c
#include "engine.h"

int engine_multiply(int a, int b) {
    return a * b;
}
```

write include/engine.h:
```c
#ifndef ENGINE_H
#define ENGINE_H

int engine_multiply(int a, int b);

#endif
```

write src/main.c:
```c
#include <stdio.h>
#include "engine.h"

int main(void) {
    printf("CLI Engine Product: 6 * 7 = %d\n", engine_multiply(6, 7));
    return 0;
}
```

write here.glue:
```glue
namespace cli {
    static_lib.add(engine)
        .sources("src/engine.c")
        .include.public("include")

    exec.add(calc)
        .sources("src/main.c")
        .depends(cli::engine)
}
```

then run:
glue run calc

it automatically compiles the engine library, links it to calc, and runs it!
