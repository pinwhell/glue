# Consume a Remote Package from GitHub

**Scenario:** I am building a C application that needs to parse INI configuration files. I want to consume the `inih` library hosted on GitHub (`github.com/pinwhell/inih`) directly in Glue, without manually writing Makefiles, configuring Git submodules, or copying third-party `.c` and `.h` files into my project tree.

---

## 1. Describe the Application and Import

In the root of the project, create `here.glue`:

```glue
import inih from github@pinwhell/inih version "^0.5.0"

namespace app;

exec.add(ini_parser)
    .sources("src/main.c")
    .depends(inih::inih)
```

### What This Declares
- **`import inih from github@pinwhell/inih`**: Glue recognizes the `github@` provider, acquires the repository from GitHub, and isolates it into `.glue/cache/sources/github.com/pinwhell/inih`.
- **`namespace app`**: Scopes our local executable target.
- **`.depends(inih::inih)`**: Declares a direct dependency on the library target exported by the imported package. Glue automatically inherits the library's public include path and linker outputs.

---

## 2. Write the Application Code

`src/main.c`:

```c
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
```

Notice that `#include "ini.h"` works directly without hardcoded `-I` flags in the source code or compiler scripts.

---

## 3. Run the Application

Execute from the project directory:

```bash
glue run ini_parser
```

### What Glue Does Under the Hood
1. **Source Provider Resolution:** Inspects `github@pinwhell/inih`. If not already cached, it fetches the repository into `.glue/cache/sources/github.com/pinwhell/inih`.
2. **Context Evaluation:** Discovers the package's `gluefile`, evaluates the target model, and registers `inih::inih`.
3. **Transitive Inclusion:** Adds the public include directory of `inih` to `ini_parser`'s compilation units.
4. **Build & Link:** Compiles `ini.c` into `ini.obj` (or `ini.o`), archives it into `libinih.a` (or `inih.lib`), compiles `src/main.c`, and links the final executable.
5. **Execution:** Runs `ini_parser` and streams the output:

```text
=== Parsing INI Config using github@pinwhell/inih ===
[server] host = 127.0.0.1
[server] port = 8080
[database] name = production
Successfully parsed configuration.
```

---

## 4. Key Guarantees
- **No Git Submodule Clutter:** The third-party repository lives cleanly in the project's `.glue/cache/` directory.
- **Qualified Scoping:** The imported library targets are scoped under `inih::`, eliminating collision risk with local targets named `inih`.
- **Target Linkage:** The consumer depends on the target symbol `inih::inih`, never on platform-dependent filenames like `libinih.a` or `inih.lib`.
