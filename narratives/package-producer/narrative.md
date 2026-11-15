# Produce a reusable native package

**Scenario:** I maintain a small C math library. I want another Glue project to
consume the compiled library and its public header without knowing its build
directory or guessing an archive filename.

This is an executable story for the current local package workflow. It uses a
static library built for the host and the selected host toolchain. Cross
compilation, Android ABIs, shared-library runtime deployment and remote
publication are described by the architecture but are not implemented by this
workflow yet.

## 1. Write the public API and implementation

`include/math.h`:

```c
#ifndef MATH_H
#define MATH_H

int math_add(int left, int right);

#endif
```

`src/math.c`:

```c
#include "math.h"

int math_add(int left, int right) {
    return left + right;
}
```

## 2. Describe the library target

`here.glue`:

```glue
when imported {
    return package()
}

namespace math;

static_lib.add(core)
    .sources("src/math.c")
    .include.public("include")
```

The imported branch is the architecture's declarative routing syntax. In the
current implementation Glue recognizes and consumes this top-level form; the
local import resolver selects a built product first, or parses the source
project when no compatible product exists. This is not general-purpose
conditional execution.

## 3. Build and produce the package

From this project directory, run:

```sh
glue package math::core --version 1.0.0
```

Glue builds the static target, stages its public headers and archive, creates a
variant manifest and detached SHA-256 digest, and places the completed package
under `product/packages/`. The current variant identity is based on the host
OS and architecture, toolchain identity and package build inputs. It does not
yet model the full ABI/runtime matrix described by the architecture.

The product follows this canonical layout:

```text
product/
  build/<build-key>/
    bin/
      <built-binaries>
    obj/
      <intermediate-objects>
    build.ninja
  packages/math/core/1.0.0/
    gluefile
    metadata.json
    variants/<variant-id>/
      gluefile
      package.toml
      content.sha256
      include/math.h
      lib/<built-static-library>
```

- **`product/build/`**: Contains the local build-tree outputs (intermediate `.obj`/`.o` files and local binary targets). Running `glue clean` purges this directory while preserving packaged products.
- **`product/packages/`**: Contains relocatable, completed package products and index manifests. The version-level `gluefile` and `metadata.json` serve as the package entry, and `variants/<variant-id>` houses the self-contained headers, libraries, and detached `content.sha256` digest. Running `glue clean --all` removes both `product/` and `.glue/`.
- **`.glue/`**: Ephemeral scratch cache for planner states and remote downloads.

The version-level `gluefile` is the import entry. The variant-level `gluefile`
describes the prebuilt target with paths relative to the variant directory.
The package payload therefore does not refer to `src/` or `.glue/` paths.

## 4. Cleaning intermediate builds without losing packages

To clean intermediate build products while preserving packaged libraries:

```sh
glue clean
```

This removes `product/build/` while keeping `product/packages/` intact. To reset everything completely, run:

```sh
glue clean --all
```

## 5. Consume it from another project

See [package-consumer](../package-consumer/narrative.md). That project imports
this producer checkout, and Glue resolves its compatible local product before
falling back to source.

**Expected guarantee:** this first workflow creates a reusable local package
product, but the consumer still needs the import path to find the producer's
`product/packages/` directory. A standalone moved/copied package import path,
remote registry and lockfile integration are future provider work.
