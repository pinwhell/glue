# Select a package variant or build from source

**Scenario:** I maintain a compression library for desktop and Android. A
consumer should use a compatible precompiled package when one exists. If it
does not, Glue should build the pinned source for the requested target rather
than accidentally linking a binary for another operating system or ABI.

## Intended target-aware behavior

The architecture gives each target configuration its own variant identity.
For example:

| Target | Example variant dimensions |
|---|---|
| Linux x86_64 | OS, architecture, libc/runtime baseline, toolchain/ABI |
| Windows x64 | OS, architecture, MSVC/MinGW ABI, CRT and linkage |
| Android `arm64-v8a` | Android ABI, NDK/API level, runtime and toolchain |
| Android `x86_64` | A separate Android ABI variant |

The resolver should use only a variant whose declared target and ABI contract
matches. A Linux ARM64 library must never stand in for Android ARM64 merely
because the CPU architecture is similar. When a compatible product is absent,
the source recipe may build that target if the required compiler, SDK and
target toolchain are available. Otherwise Glue should report what is missing.

**This matrix is planned, not implemented yet.** The current package producer
and consumer support host-native static libraries only. They do not select
Android ABIs, cross-compile, validate libc/CRT compatibility, or build multiple
target variants. The following runnable fallback story demonstrates the
implemented host-only source path.

## Current runnable story: no product yet, use the source checkout

The `source-project` folder below contains `here.glue`, `include/codec.h` and
`src/codec.c`, but it has no `product/packages/` output initially. The
consumer imports that source project and depends on the exported target:

`fallback-consumer/here.glue`:

```glue
import compress from "../source-project"

namespace demo;

exec.add(compress_demo)
    .sources("src/main.c")
    .depends(compress::core)
```

`fallback-consumer/src/main.c`:

```c
#include <stdio.h>
#include "codec.h"

int main(void) {
    printf("encoded size: %d\n", codec_encoded_size(12));
    return 0;
}
```

Run `glue run compress_demo` in `fallback-consumer`. With no compatible local
product in `source-project/product/packages/`, Glue parses the source project's target,
builds its static library into the consumer's `product/build/<build-key>/`, and links
the executable. After a successful build/run, Glue caches the host package in
`source-project/product/packages/` for later local consumers.

The source target is not version-pinned by this local-path flow, and the
current resolver does not use `when imported` as a full context evaluator; it
recognizes the routing form while resolving an import and handles the local
source fallback directly. Remote source revisions, lockfiles and controlled
package-build contexts remain architecture work.

## What a later Android run should do

Once Android target support exists, the same dependency should resolve to an
Android ABI-specific variant or build from the pinned source recipe with an
NDK toolchain. If that is impossible, Glue should name the missing ABI/toolchain
requirement and stop before linking. The current CLI cannot perform this step;
do not use this story as evidence that Android packaging works today.
