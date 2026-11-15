# Package and Consume a Shared Library (`shared_lib`)

**Scenario:** I maintain a high-performance native math library (`fastmath`) compiled as a shared library (`shared_lib` / DLL / SO). I want consumer applications to link against its interface and execute seamlessly without manual DLL copying, missing loader errors, or environment variable manipulation.

This executable narrative validates Section §7.2 ("Producing Shared Library Packages") and Section §14.3 ("Packaging and Consuming a Shared Library") of the Glue Architecture Specification.

---

## 1. Producer Project (`fastmath`)

The producer library defines an exported C API with cross-platform export macros.

### 1.1. Public Header (`fastmath/include/fastmath.h`)

```c
#ifndef FASTMATH_H
#define FASTMATH_H

#if defined(_WIN32) || defined(__CYGWIN__)
  #if defined(FASTMATH_EXPORTS)
    #define FASTMATH_API __declspec(dllexport)
  #else
    #define FASTMATH_API __declspec(dllimport)
  #endif
#else
  #if defined(__GNUC__) && __GNUC__ >= 4
    #define FASTMATH_API __attribute__((visibility("default")))
  #else
    #define FASTMATH_API
  #endif
#endif

#ifdef __cplusplus
extern "C" {
#endif

FASTMATH_API int fastmath_multiply(int a, int b);
FASTMATH_API int fastmath_power(int base, int exp);

#ifdef __cplusplus
}
#endif

#endif /* FASTMATH_H */
```

### 1.2. Implementation (`fastmath/src/fastmath.c`)

```c
#include "fastmath.h"

FASTMATH_API int fastmath_multiply(int a, int b) {
    return a * b;
}

FASTMATH_API int fastmath_power(int base, int exp) {
    int result = 1;
    for (int i = 0; i < exp; ++i) {
        result *= base;
    }
    return result;
}
```

### 1.3. Producer Manifest (`fastmath/here.glue`)

```glue
when imported {
    return package()
}

namespace math;

shared_lib.add(fastmath)
    .sources("src/fastmath.c")
    .include.public("include")
    .define("FASTMATH_EXPORTS")
```

---

## 2. Produce the Shared Library Package

From the `fastmath` directory, produce the relocatable binary package:

```sh
cd fastmath
glue package math::fastmath --version 2.0.0
```

### What Glue Executes:
1. **Compilation:** Compiles `src/fastmath.c` with MSVC `/MD` and define `FASTMATH_EXPORTS`.
2. **Linkage (`link_shared`):** MSVC `link.exe /DLL` links `fastmath.dll` and generates the corresponding import library `fastmath.lib`.
3. **Partitioned Staging:**
   - Link-time interface staged to `stage/lib/fastmath.lib`.
   - Runtime deployment payload staged to `stage/bin/fastmath.dll`.
   - Public headers staged to `stage/include/fastmath.h`.
4. **Digest Calculation:** Computes the canonical SHA-256 tree digest over all staged files.
5. **Atomic Commit:** Commits the variant to `product/packages/math/fastmath/2.0.0/variants/<variant-id>/`.

### Resulting Package Topology:
```text
product/packages/math/fastmath/2.0.0/
├── gluefile              # Entry manifest: when imported { return package() }
├── metadata.json         # Variant index and SHA-256 signatures
└── variants/
    └── <variant-id>/
        ├── content.sha256
        ├── gluefile      # Variant-level bindings
        ├── package.toml  # Variant metadata and runtime descriptors
        ├── include/
        │   └── fastmath.h
        ├── lib/
        │   └── fastmath.lib    # PE Import Library (link-time interface)
        └── bin/
            └── fastmath.dll    # Dynamic runtime payload
```

The emitted variant `gluefile` explicitly records both dimensions:
```glue
namespace math;
shared_lib.add(fastmath)
    .prebuilt("lib/fastmath.lib")
    .runtime("bin/fastmath.dll")
    .include.public("include")
```

---

## 3. Consumer Project (`simulation`)

The consumer application resides alongside the producer and consumes `fastmath::fastmath`.

### 3.1. Consumer Implementation (`simulation/src/main.c`)

```c
#include <stdio.h>
#include "fastmath.h"

int main(void) {
    int mul = fastmath_multiply(6, 7);
    int pow = fastmath_power(2, 8);
    printf("FastMath Multiply: 6 * 7 = %d\n", mul);
    printf("FastMath Power: 2 ^ 8 = %d\n", pow);
    return 0;
}
```

### 3.2. Consumer Manifest (`simulation/here.glue`)

```glue
import fastmath from "../fastmath"

namespace app;

exec.add(sim)
    .sources("src/main.c")
    .depends(fastmath::fastmath)
```

---

## 4. Run the Consumer with Automatic Runtime Deployment

From the `simulation` directory, run:

```sh
cd ../simulation
glue run sim
```

### Execution Output:
```text
glue: Target artifact missing, building 'sim'...
glue: Staged runtime artifact: fastmath.dll
FastMath Multiply: 6 * 7 = 42
FastMath Power: 2 ^ 8 = 256
```

### What Glue Did Automatically:
1. **Resolution:** Located the precompiled variant under `../fastmath/product/packages/math/fastmath/2.0.0/`.
2. **Link Step:** Supplied `lib/fastmath.lib` to MSVC `link.exe` to resolve symbols for `sim.exe`.
3. **Deployment Step (Automatic Closure):** Discovered `bin/fastmath.dll` in `fastmath`'s runtime artifacts and copied it into `simulation/product/build/x86_64-windows/bin/fastmath.dll` alongside `sim.exe`.
4. **Execution:** Launched `sim.exe`. The Windows dynamic loader found `fastmath.dll` immediately in the executable's directory, running cleanly with zero loader errors.
