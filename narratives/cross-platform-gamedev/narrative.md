i am an indie game developer writing a 2d game in c
i want my game binary to link against a rendering module
the rendering module should expose its public header to the game without messing with manual include directories

write engine/renderer.h:
```c
#ifndef RENDERER_H
#define RENDERER_H

void render_frame(int frame_number);

#endif
```

write engine/renderer.c:
```c
#include <stdio.h>
#include "renderer.h"

void render_frame(int frame_number) {
    printf("[GameEngine] Rendered frame #%d successfully.\n", frame_number);
}
```

write game/main.c:
```c
#include <stdio.h>
#include "renderer.h"

int main(void) {
    puts("Starting game loop...");
    for (int i = 1; i <= 3; i++) {
        render_frame(i);
    }
    puts("Game loop complete.");
    return 0;
}
```

write here.glue:
```glue
namespace engine;

static_lib.add(render)
    .sources("engine/renderer.c")
    .include.public("engine")

exec.add(game)
    .sources("game/main.c")
    .depends(engine::render)
```

run:
glue run game

and it compiles both, links them, and plays out the game loop!
