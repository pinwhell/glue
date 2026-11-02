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
