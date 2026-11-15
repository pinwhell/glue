<div align="center">

# Glue

### The build system and package manager C and C++ always deserved.

*Inspired by the ergonomics of **Rust's Cargo**. Powered by the raw speed of **Ninja**.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)]()
[![Toolchains](https://img.shields.io/badge/toolchains-MSVC%20%7C%20Clang%20%7C%20GCC-purple.svg)]()

</div>

---

## What is Glue?

If you have ever used **Rust's Cargo**, **Go**, or **npm**, you know how good modern tooling feels: you create a project, declare your dependencies, hit `run`, and everything just works.

In C and C++, we have spent decades wrestling with:
- Arcane 30-year-old **CMake** scripts and macro syntax.
- **"Missing DLL" errors** (`fastmath.dll was not found`) every time you try to run an application.
- Hunting down compiler environment scripts (`vcvarsall.bat`, custom sysroots).
- Copy-pasting `.c` and `.h` files into `vendor/` directories because there is no simple way to import a library.
- Dependency builds compiling 200 unit tests and benchmarks you never asked for.

**Glue solves this.** It gives C and C++ developers a modern, intuitive, declarative workflow with zero configuration, automatic dependency resolution, and instant execution.

---

## Why Glue? (Solving C/C++'s Biggest Pain Points)

| The Pain in Traditional C/C++ | How Glue Solves It |
|---|---|
| **CMake Boilerplate Hell:** Writing 30 lines of `cmake_minimum_required`, `add_executable`, `target_include_directories`, and `set_target_properties` just to compile two files. | **3 Lines of Declarative Code:**<br>`exec.add(app).sources("src/*.c").include.public("include")`<br>Clean, readable, and intuitive. |
| **"Missing DLL" Loader Crashes:** You compile an executable with a shared library, run it, and the OS crashes because the `.dll` or `.so` is not in your path. | **Automatic Runtime Deployment:** Glue discovers all shared libraries in your dependency tree and stages the `.dll`/`.so` directly alongside your executable. `glue run` just works. |
| **No Simple Package Imports:** Having to manually clone repositories, configure git submodules, or struggle with complex package managers. | **Direct Git & GitHub Imports:**<br>`import inih from github@pinwhell/inih`<br>Glue clones, caches, links, and manages it for you. |
| **Subproject Pollution:** Including a library via `add_subdirectory()` drags in the author's internal tests, examples, and linters into your build. | **Declarative Context Separation (`when imported`):** Libraries cleanly separate their internal developer tests from what they export to consumers. |
| **Manual Compiler Setup:** Forgetting to run Visual Studio's `vcvarsall.bat` before compiling on Windows. | **Automatic Toolchain Discovery:** Glue auto-detects installed compilers (MSVC Community, Clang, GCC) and sets up their environments automatically. |
| **Non-Reproducible Builds:** Builds breaking because a teammate has a different library version installed globally. | **Deterministic Lockfile (`glue.lock`):** Pinned versions, variant hashes, and SHA-256 digests ensure 100% reproducible builds across machines. |

---

## Quick Start (60 Seconds)

### 1. Installation

Build the single self-contained `glue` binary with Rust:

```bash
git clone https://github.com/pinwhell/glue.git
cd glue
cargo install --path crates/glue-cli
```

*(Ensure [Ninja](https://ninja-build.org/) 1.11+ is installed and on your PATH).*

### 2. Create Your First Project

```bash
mkdir my-app && cd my-app
glue init my_app
```

This generates a ready-to-run project:
- `here.glue`: Your project configuration.
- `main.cpp`: A starter C++ application.

### 3. Run It

```bash
glue run
```

```text
glue: Successfully built artifact: product/build/x86_64-windows/bin/my_app.exe
Hello from Glue!
```

No build directories to create, no generator flags to pass, no environment variables to configure.

---

## How Simple is the Configuration?

### A Simple Executable
`here.glue`:
```glue
namespace my_project;

exec.add(hello).sources("hello.c")
```

### A Library and a Consumer Application
`here.glue`:
```glue
namespace math {
    static_lib.add(core)
        .sources("src/math/*.c")
        .include.public("include")
}

namespace app {
    exec.add(calculator)
        .sources("src/main.c")
        .depends(math::core)
}
```
> [!TIP]
> Notice how `app` depends on `math::core`. Glue automatically forwards the public include directories and preprocessor definitions to `app` without any manual flag forwarding.

### Using Shared Libraries (DLLs / .so) with Zero Loader Errors
Building shared libraries on Windows and Linux is notoriously painful because of import libraries and runtime loader paths. In Glue, it's effortless:

**The Library (`fastmath`):**
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

**The Consumer Application (`app`):**
```glue
import fastmath from "../fastmath"

exec.add(sim)
    .sources("src/main.c")
    .depends(fastmath::fastmath)
```

Run it with:
```bash
glue run sim
```

```text
glue: Target artifact missing, building 'sim'...
glue: Staged runtime artifact: fastmath.dll
Simulation completed successfully!
```
Glue handles the link-time interface (`.lib`), copies the runtime payload (`.dll`) right next to `sim.exe`, and runs the program. **Zero missing DLL errors.**

---

## Importing Packages from GitHub

Import open-source libraries directly using intuitive package syntax:

```glue
import inih from github@pinwhell/inih version "^0.5.0"

exec.add(config_reader)
    .sources("src/main.c")
    .depends(inih::inih)
```

Glue downloads the package to `.glue/cache/`, verifies its integrity, builds it with your active compiler, and locks the exact revision in `glue.lock`.

---

## Inspired by Rust's Cargo

If you love Cargo, Glue will feel like second nature:

| Cargo Workflow | Glue Equivalent | Description |
|---|---|---|
| `cargo new foo` | `glue init foo` | Creates a new project workspace. |
| `cargo build` | `glue build` | Compiles project targets using Ninja. |
| `cargo run` | `glue run` | Compiles and executes an application immediately. |
| `cargo build --locked` | `glue build --locked` | Verifies build against `glue.lock` without updating it (ideal for CI/CD). |
| `cargo clean` | `glue clean` | Removes transient build artifacts in `product/build/`. |
| `Cargo.lock` | `glue.lock` | Deterministic lockfile tracking all dependencies and SHA-256 digests. |

---

## Clean, Predictable Directory Structure

Unlike CMake which scatters build files everywhere or leaves developers guessing where outputs landed, Glue enforces a clean, predictable topology:

```text
my-project/
├── here.glue                 # Your declarative project configuration
├── glue.lock                 # Pinned dependencies & cryptographic digests
├── src/                      # Your C / C++ source code
│   └── main.c
├── include/                  # Your public header files
│   └── my_project.h
│
├── product/                  # All generated outputs
│   ├── build/                # Ephemeral build outputs (safe to delete anytime)
│   │   └── x86_64-windows/
│   │       ├── obj/          # Intermediate .obj / .o files
│   │       └── bin/          # Linked executables & runtime DLLs
│   │
│   └── packages/             # Relocatable, versioned binary packages
│       └── my_project/0.1.0/
│
└── .glue/                    # Engine cache (bare git clones, fingerprints)
```

---

## Command Reference

Glue keeps its command-line interface clean and discoverable:

```bash
# Initialize a new project
glue init [name]

# Build the project (or a specific target)
glue build [target]

# Build and run an executable target
glue run [target] [-- args forwarded to program...]

# Build strictly against glue.lock (fails if lockfile is out of date)
glue build --locked

# Package a library into a relocatable binary package
glue package <target> --version <version>

# Inspect targets, include paths, defines, and dependencies
glue explain [target]

# Export the build DAG in Graphviz DOT format to visualize dependencies
glue graph

# Clean build artifacts
glue clean         # Removes product/build/
glue clean --all   # Purges product/ and .glue/ cache
```

---

## Self-Verifying Narratives

Glue's features are backed by executable, tested narratives in the [`narratives/`](narratives/) folder:

- [**Shared Library Packaging & Deployment:**](narratives/shared-library-package/narrative.md) Packaging a shared DLL/SO and consuming it with automatic runtime deployment.
- [**GitHub Package Import:**](narratives/github-package-import/narrative.md) Importing a remote C library from GitHub (`pinwhell/inih`).
- [**Locked Reproducible Builds:**](narratives/locked-reproducible-builds/narrative.md) Frozen builds and verification using `glue.lock`.
- [**Binary Package Production & Consumption:**](narratives/package-producer/narrative.md) Creating relocatable binary packages with detached SHA-256 digests.

---

## Built on the Shoulders of Giants

Glue is a lightweight, ergonomic orchestration layer. We are deeply grateful to:
- **[Evan Martin & the Ninja Team](https://ninja-build.org/):** For building the fastest, most reliable execution engine in software engineering.
- **[The Rust Language & Cargo Team](https://www.rust-lang.org/):** For inspiring modern package management and developer ergonomics.
- **[LLVM / Clang](https://llvm.org/) & [Microsoft MSVC](https://visualstudio.microsoft.com/):** For world-class native compiler toolchains.
- **[CMake & Meson](https://cmake.org/):** For pioneering native build paradigms.

---

## License

Glue is distributed under the [MIT License](LICENSE).
