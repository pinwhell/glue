# Consume a produced package

**Scenario:** I am writing a command-line calculator and want to use the math
library from the neighboring producer project.

First, produce the host package by following
[package-producer](../package-producer/narrative.md):

```sh
cd ../package-producer
glue package math::core --version 1.0.0
```

Then create the consumer project alongside it.

`src/main.c`:

```c
#include <stdio.h>
#include "math.h"

int main(void) {
    printf("2 + 3 = %d\n", math_add(2, 3));
    return 0;
}
```

`here.glue`:

```glue
import math from "../package-producer"

namespace app;

exec.add(calculator)
    .sources("src/main.c")
    .depends(math::core)
```

Run it from the consumer project:

```sh
glue run calculator
```

Glue searches the producer's `product/packages/`, selects a host-compatible
variant produced by the same toolchain fingerprint, verifies its digest, and
uses its exported include and archive paths to build the executable under its own
`product/build/<build-key>/bin/`. The consumer names the target `math::core`; it
does not hard-code `libcore.a`, `.lib`, or a producer build directory.

If the producer has no compatible local product, the current local resolver
falls back to compiling the producer's source target in the consumer's build
graph. After a successful consumer build/run, Glue attempts to package that
source-built static target at the producer so a later consumer can reuse it.
That promotion is best-effort today; it is not a substitute for a pinned source
revision, lockfile or explicit package build recipe.

The current import path points to the producer checkout. The package payload
itself is staged with relative paths, but discovering a moved product without
the producer checkout requires the future direct-product/provider interface.
