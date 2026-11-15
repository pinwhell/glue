# Deterministic Builds with `glue.lock` and `--locked`

**Scenario:** I am deploying my application in continuous integration (CI) and production environments. I want to guarantee that builds are bit-for-bit reproducible and frozen against unexpected dependency drift, using `glue.lock` and `glue build --locked`.

---

## 1. Project Configuration

In the project root, create `here.glue`:

```glue
import inih from github@pinwhell/inih version "^0.5.0"

namespace server;

exec.add(service)
    .sources("src/main.c")
    .depends(inih::inih)
```

`src/main.c`:

```c
#include <stdio.h>
#include "ini.h"

int main(void) {
    printf("Locked deterministic build service running.\n");
    return 0;
}
```

---

## 2. Generating the Dependency Lockfile

During local development, running `glue build` automatically records the exact resolved state into `glue.lock`:

```bash
glue build
```

This generates `glue.lock` in the project root:

```toml
version = 1

[[package]]
coordinate = "github@pinwhell/inih"
version = "^0.5.0"
delivery_mode = "source_built"
```

### What `glue.lock` Stores
- **`coordinate`**: The canonical package origin (e.g. `github@pinwhell/inih` or local vendor path).
- **`version`**: The exact resolved version or constraint.
- **`delivery_mode`**: How the package was satisfied (`local_product`, `binary_download`, or `source_built`).

---

## 3. Enforcing Frozen Builds in CI/CD (`--locked`)

In CI/CD environments, builds should never silently update the lockfile or accept undeclared dependencies. Running:

```bash
glue build --locked
```

Output:
```text
glue: Verified dependency resolution against glue.lock
glue: Building target 'server::service' using MSVC (Microsoft Visual C/C++)
glue: Successfully built artifact: .../service.exe
```

If a developer adds a new dependency in `here.glue` but forgets to commit an updated `glue.lock`, Glue immediately halts with a frozen lockfile mismatch:

```text
glue error: Frozen lockfile mismatch: package 'logger' (github@corp/logger) is not recorded in glue.lock
```

---

## 4. Key Guarantees
- **Zero Drift:** CI builds only execute when dependencies match the audited `glue.lock`.
- **Reproducibility:** Prevents "it worked on my machine" issues across distributed teams.
- **Auditable Supply Chain:** Every dependency, delivery mode, and version constraint is committed directly in version control.
