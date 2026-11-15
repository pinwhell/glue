#include <stdio.h>
#include "ini.h"

static int handler(void* user, const char* section, const char* name, const char* value) {
    (void)user;
    printf("[%s] %s = %s\n", section, name, value);
    return 1;
}

int main(void) {
    const char* config_text = 
        "[server]\n"
        "host = 127.0.0.1\n"
        "port = 8080\n"
        "[database]\n"
        "name = production\n";

    printf("=== Parsing INI Config using github@pinwhell/inih ===\n");
    if (ini_parse_string(config_text, handler, NULL) < 0) {
        printf("Failed to parse configuration string.\n");
        return 1;
    }
    printf("Successfully parsed configuration.\n");
    return 0;
}
