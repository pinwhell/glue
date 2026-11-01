# Glue

Glue is a unified native development system for C and C++ projects. It connects declarative project definition, toolchain abstraction, build planning, and execution behind a single consistent CLI.

Instead of orchestrating compilers, linkers, environment scripts, and multi-file configurations manually, Glue resolves the target dependency graph and delegates direct execution to proven high-performance backends.

---

## Quick Start

### 1. Installation & Build

Build the CLI from source using Cargo:

```bash
cargo build --release
```

The resulting binary is located at `target/release/glue` (or `glue.exe` on Windows).

### 2. Define a Project (`gluefile`)

A minimal executable:

```glue
exec.add(hello).sources("hello.c")
```

A reusable library and consuming executable:

```glue
static_lib.add(core)
    .sources("src/core.c")
    .include.public("include")

exec.add(app)
    .sources("src/main.c")
    .depends(core)
```

### 3. Build & Run

```bash
# Build a specific target
glue build app

# Build (if necessary) and execute
glue run app

# Inspect dependency hierarchy
glue explain app

# Visualize the build DAG in Graphviz DOT format
glue graph

# Clean build artifacts
glue clean
```

---

## Architecture

Glue is organized as a modular Rust workspace:

- [`crates/glue-model`](crates/glue-model): Unified target model (`exec`, `static_lib`, `shared_lib`, `object_lib`, `interface_lib`, `custom`), build profiles, and project representation.
- [`crates/glue-config`](crates/glue-config): Lexer and parser for the declarative Glue DSL (`gluefile`, `here.glue`).
- [`crates/glue-toolchains`](crates/glue-toolchains): Native compiler abstractions (MSVC `cl`/`link`/`lib`, Clang, GCC, and environment autodetection).
- [`crates/glue-build`](crates/glue-build): Graph planner, transitive dependency ordering, include propagation, and SHA-256 fingerprinting.
- [`crates/glue-backend-ninja`](crates/glue-backend-ninja): High-speed DAG code generation and execution via Ninja.
- [`crates/glue-packages`](crates/glue-packages): Package manifests and remote/local provider definitions.
- [`crates/glue-resolver`](crates/glue-resolver): Dependency resolution and `glue.lock` serialization.
- [`crates/glue-platform-android`](crates/glue-platform-android): NDK discovery and ABI triples (`arm64-v8a`, `armeabi-v7a`, `x86_64`, `x86`).
- [`crates/glue-cli`](crates/glue-cli): CLI entry point.

---

## Acknowledgments & Credits

Glue is a lightweight orchestration layer that stands on the shoulders of decades of engineering by open-source communities and compiler engineers. All credit for the underlying execution, compilation, and dependency technology belongs to:

- **[Ninja](https://ninja-build.org/)** (Evan Martin, Google, and the Ninja maintainers) for creating the definitive high-performance build execution engine.
- **[LLVM / Clang](https://llvm.org/)** (Chris Lattner and the LLVM Foundation) for state-of-the-art compiler and linker infrastructure.
- **[Microsoft Visual C++ (MSVC)](https://visualstudio.microsoft.com/)** team for Windows native toolchain standards and developer tooling.
- **[The Rust Language & Cargo](https://www.rust-lang.org/)** (The Rust Foundation and contributors) for memory-safe systems programming and ergonomic package workflows.
- **[CMake](https://cmake.org/) (Kitware), [Meson](https://mesonbuild.com/), and [Bazel](https://bazel.build/)** communities for pioneering build paradigms and dependency patterns that inform native software engineering.

---

## License

MIT &copy; Glue Contributors
