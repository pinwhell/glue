# Glue
## A Unified Native Development System

**Architecture & Design Specification**  
**Version:** 0.3  
**Status:** Proposed architecture  
**Scope:** C, C++, package management, native build orchestration, toolchains, cross-compilation and ABI compatibility. Rust is a planned extension, not a prerequisite for the initial implementation.

---

# 1. Abstract

Glue is a native development system designed to unify project configuration, dependency management, compilation, linking and execution behind a consistent interface.

Its design draws inspiration from CMake's target-oriented build model and the dependency-management experience of npm and Composer. Its goal is to make native development more approachable without sacrificing the flexibility required by real-world C and C++ projects.

Glue introduces a declarative project language, a unified target model, a package resolution system, a dependency lockfile, a graph-based build engine, configurable toolchains and a target-aware execution model.

A project should be able to define an executable, import dependencies, select a compiler and target platform, and run the resulting program without manually coordinating the underlying tools.

The same architecture should accommodate advanced requirements, including custom compilers, generated sources, cross-compilation, SDK integration, binary compatibility constraints, Android native development and interoperability between languages.

Glue is not intended to replace every existing compiler, SDK or specialized build tool. It aims to coordinate them through a common project model and a predictable developer experience.

The initial implementation prioritizes C and C++. Rust integration, advanced package distribution, remote build caching and broader platform support are designed as extensions of the core architecture.

The central architectural principle is simple:

**Describe what should be built, resolve what it requires, determine how the target can be built, and execute the resulting build plan.**

# 2. Vision and Scope

## 2.1. Product vision

Glue aims to provide one coherent workflow for defining, building, managing and running native software.

A minimal project should be straightforward:

```glue
exec.add(hello).sources("hello.c")
```

The developer should then be able to execute:

```bash
glue run hello
```

Glue should resolve the project configuration, compile the necessary sources and launch the resulting executable.

As the project grows, the same model should support libraries, external dependencies, multiple compilers, different build configurations and alternative target platforms.

For example:

```glue
import json from github@alice/json

exec.add(app)
    .sources("src/main.cpp")
    .depends(json)
```

The intended experience is not merely a shorter syntax for compiler commands. It is a consistent way to express the relationships between source files, targets, packages, toolchains and platforms.

## 2.2. What Glue is

Glue is best understood as a **native development system with an integrated build engine and package manager**.

Its primary responsibilities are:

- Project configuration.
- Target and dependency modeling.
- Native compilation and linking.
- Package acquisition and resolution.
- Incremental build planning.
- Toolchain selection and integration.
- Cross-compilation configuration.
- Artifact compatibility validation.
- Program execution and test orchestration.

## 2.3. What Glue is not

The initial architecture does not require Glue to implement its own compiler, linker, operating-system SDK or runtime.

It should use existing tools wherever possible.

It also should not initially attempt to replace complete application development ecosystems such as Android's application packaging workflow.

Glue can integrate with those systems without assuming responsibility for every part of them.

# 3. Design Principles

## 3.1. One coherent project model

Targets, dependencies, compiler requirements and platform constraints should be represented in a common model.

Separate subsystems may implement different responsibilities, but they must agree on the meaning of a target, package identity and artifact.

## 3.2. Simple by default

A basic executable should require as little configuration as possible.

Advanced features should remain available without forcing every project to declare toolchains, sysroots, ABI settings or compiler arguments explicitly.

## 3.3. Explicit when necessary

When automatic resolution becomes ambiguous, Glue should require explicit configuration rather than silently selecting an incompatible compiler, package or artifact.

Defaults should reduce configuration, not conceal meaningful decisions.

## 3.4. Reproducibility by design

Package versions, source revisions, compiler identities, target properties and build inputs should be represented explicitly enough to reproduce builds under equivalent conditions.

Reproducibility must be treated as an architectural concern rather than a feature added after caching.

## 3.5. Existing tools remain first-class

Glue should orchestrate compilers and linkers through adapters.

Users should retain access to compiler-specific options, SDK functionality and specialized workflows when the generic model is insufficient.

## 3.6. Separate identity from naming

A package's display name must not determine its identity.

Likewise, a target's name, a binary's filename and the identity of the package exporting that target are distinct concepts.

## 3.7. Stable boundaries, extensible implementations

The public configuration language, internal project model, build engine and toolchain interfaces should evolve independently whenever possible.

## 3.8. Correctness before optimization

The first build engine should prioritize correct dependency tracking, reliable invalidation and accurate diagnostics.

Parallelism and caching should be introduced without compromising those guarantees.

# 4. System Architecture

Glue should be organized as a sequence of transformations between well-defined representations.

The complete workflow is:

```text
Project Configuration
        |
        v
Lexer and Parser
        |
        v
Configuration AST
        |
        v
Semantic Validation
        |
        v
Normalized Project Model
        |
        v
Dependency Resolution
        |
        v
Resolved Project Model
        |
        v
Target and Toolchain Resolution
        |
        v
Build Graph Planner
        |
        v
Build Action Graph
        |
        v
Scheduler and Executor
        |
        v
Artifacts
        |
        v
Run / Test / Install / Package
```

Not every command needs to execute every stage.

For example, a command that inspects a project should not need to compile it. A dependency-resolution command should not launch an executable.

## 4.1. Core components

The architecture consists of the following logical components.

| Component | Responsibility |
|---|---|
| Configuration frontend | Parses and validates project declarations |
| Project model | Represents targets, properties and relationships |
| Package resolver | Resolves package identities and versions |
| Source providers | Acquires source packages and repository revisions |
| Lockfile manager | Records and verifies resolved dependencies |
| Target resolver | Determines platform and ABI requirements |
| Toolchain manager | Discovers and selects compatible tools |
| Build planner | Converts targets into build actions |
| Scheduler | Orders and parallelizes independent actions |
| Action executor | Runs compilers, linkers and generators |
| Artifact store | Tracks outputs and caches reusable results |
| CLI | Exposes build, run, test, clean and inspection commands |

These components should initially be logical modules. They do not need to be separate processes or independently distributed packages.

# 5. Project Configuration Language

Glue should provide a compact declarative language designed specifically for native project configuration.

The language should be expressive enough for common build requirements while remaining easy to parse, validate and inspect.

## 5.1. Basic syntax

A minimal executable:

```glue
exec.add(hello).sources("hello.c")
```

An expanded equivalent:

```glue
exec.add(hello)
    .sources("hello.c")
```

A library:

```glue
static_lib.add(core)
    .sources("src/core.c")
    .include.public("include")
```

An executable consuming that library:

```glue
exec.add(app)
    .sources("src/main.c")
    .depends(core)
```

The language should support chained declarations without making chaining mandatory.

## 5.2. Initial target declarations

The initial target types should include:

| Target type | Output |
|---|---|
| `exec` | Executable |
| `static_lib` | Static library |
| `shared_lib` | Shared library |
| `object_lib` | Reusable object files |
| `interface_lib` | Propagated usage requirements |
| `custom` | Generated artifacts or custom operations |

The exact public names can change during language design, but their semantics must be stable.

## 5.3. Sources

Targets should accept explicit source declarations:

```glue
exec.add(app)
    .sources(
        "src/main.cpp",
        "src/parser.cpp"
    )
```

Glue should also support generated sources.

### Glob patterns

The declarative language supports file globbing patterns (e.g., `"src/*.c"`) to match groups of source files ergonomically:

```glue
static_lib.add(core)
    .sources("src/*.c")

exec.add(app)
    .sources(
        "src/*.cpp",
        "generated/parser.cpp"
    )
```

**Resolution semantics for glob patterns:**
- Patterns such as `"src/*.c"`, `"engine/*.cpp"`, and `"tests/test_*.c"` are evaluated relative to the project root directory during project configuration.
- Glob expansion results are deterministically sorted alphabetically by path to guarantee reproducible build plans across machines and operating systems.
- Unmatched glob patterns produce an early diagnostic before scheduling build actions, preventing silent compilation omissions.

### Regular expression patterns

In succession with glob patterns, Glue supports regular expression matching for fine-grained source discovery (such as versioned source files, complex naming conventions, or filtered subsets):

```glue
static_lib.add(core)
    .sources("regex:src/core_v[0-9]+\\.c")

exec.add(app)
    .sources(
        "regex:src/v[0-9]+_.*\\.cpp",
        "generated/parser.cpp"
    )
```

**Resolution semantics for regex patterns:**
- Regex source specifications begin with the `regex:` prefix (or `.sources.regex(...)`) and evaluate an expression against candidate files within the specified search directory.
- Candidate files are discovered in the base path and filtered against the regular expression.
- Matched file lists are deterministically sorted alphabetically by path to maintain reproducible build graphs across platforms.
- If a regex pattern matches zero files, Glue raises an early configuration diagnostic to prevent unexpected empty targets.

Automatic source discovery may be offered as an optional convenience, but explicit source declarations, globs and regex patterns remain the predictable default.

This avoids unexpectedly adding files to a build simply because they appeared in a directory.

## 5.4. Include directories and definitions

Include directories should have explicit visibility:

```glue
static_lib.add(core)
    .include.private("src/internal")
    .include.public("include")
```

Compile definitions should be similarly scoped:

```glue
exec.add(app)
    .define("APP_VERSION", "1")
```

Glue must distinguish compile properties from link properties and avoid propagating settings indiscriminately.

## 5.5. Dependency declarations

Targets should express dependencies directly:

```glue
exec.add(app)
    .sources("src/main.cpp")
    .depends(core)
```

A dependency reference should resolve to a specific target identity during semantic validation.

An unresolved reference should produce an error before compilation begins.

## 5.6. Configuration language semantics

The configuration language should initially support:

- Identifiers.
- Strings.
- Numbers and booleans.
- Lists.
- Function and method calls.
- Target declarations.
- Dependency imports.
- Build configuration declarations.
- Comments.

The language should not require arbitrary imperative execution to express ordinary project configuration.

This enables static validation, deterministic parsing and future IDE support.

## 5.7. Syntax and semantics are separate

The following declarations should produce equivalent internal models:

```glue
exec.add(app).sources("main.c")
```

```glue
exec.add(app)
    .sources("main.c")
```

The parser should produce an abstract syntax tree. Semantic validation should then transform it into a normalized project representation.

The build engine should never depend directly on formatting or syntax choices.

## 5.8. Namespaces and target qualification

Glue supports namespaces to prevent target name collisions across modular projects, subprojects, and imported third-party packages.

### Namespace declarations

Projects can declare a namespace to scope their exposed targets:

```glue
namespace openssl;

static_lib.add(crypto)
    .sources("src/crypto.c")
    .include.public("include")
```

Alternatively, scoped blocks may group targets under explicit namespaces:

```glue
namespace openssl {
    static_lib.add(ssl)
        .sources("src/ssl.c")
        .include.public("include")
}
```

### Qualified target references

Consumers refer to namespaced targets using double-colon qualification syntax:

```glue
import openssl from "../openssl"

exec.add(client)
    .sources("src/client.c")
    .depends(openssl::ssl)
```

If a package exports a primary target matching its package name, or if aliasing is configured, Glue resolves both qualified (`openssl::ssl`) and package-level references (`openssl`) according to project-level visibility rules.

# 6. Package Management

Glue's package manager should make local and remote dependencies available through a consistent interface.

It should support source packages first, while allowing compatible binary packages to be introduced without changing the project's dependency model.

## 6.1. Package sources

The initial source types should be:

**GitHub repositories**

```glue
import json from github@alice/json
```

**Local directories**

```glue
import core from vendor@../core
```

**Arbitrary Git repositories**

```glue
import compression from git@"https://example.org/compression.git"
```

These examples represent proposed syntax.

Each source provider should implement a common acquisition interface, allowing additional providers and registries to be introduced later.

## 6.2. Version constraints

Remote packages should support explicit version constraints:

```glue
import json from github@alice/json version "^2.0"
```

A version constraint describes acceptable releases.

The resolver selects a version satisfying the constraints, then records the exact immutable revision used by the project.

The selected version and exact revision must not be treated as interchangeable.

## 6.3. Package identity

A package identity should distinguish at least:

- Source provider.
- Repository or registry identity.
- Package identity within that source.
- Resolved revision.
- Relevant package metadata.

Two packages with the same display name but different identities must be distinguishable.

The implementation should use structured identities rather than relying on string concatenation throughout the codebase.

## 6.4. Namespaces

Packages should expose qualified target names.

For example:

```text
alice_json::json
bob_json::json
```

These names are illustrative.

Glue should allow a consumer to define a local alias without changing the canonical package identity.

Namespaces prevent ambiguity in the build model, but they do not automatically eliminate symbol collisions in compiled C or C++ binaries.

### Resolution and Scoping Rules

1. **Canonical Qualification:** Every target in the project graph has a canonical name `<namespace>::<target>`. Targets declared without an explicit namespace default to the project's root namespace.
2. **Local Aliases:** Consumers may bind local aliases during import to avoid deep hierarchies or resolve local naming preferences:
   ```glue
   import json from github@alice/json as local_json
   ```
3. **Collision Isolation:** Two packages exporting identically named internal targets (such as `core` or `utils`) coexist cleanly in the build graph because their graph identities are partitioned by their package namespaces.
4. **Symbol Visibility vs Target Scoping:** Target namespaces ensure unambiguous DAG operations and linker inputs, but native C/C++ global symbols still require appropriate compiler visibility attributes or namespace encapsulation to prevent runtime symbol collisions.

## 6.5. Package manifests

Each distributable package should have a manifest describing:

- Package identity and version.
- Exported targets.
- Source files or artifact metadata.
- Public usage requirements.
- Dependencies.
- Supported platforms.
- Package metadata format version.

A proposed filename is `glue.package`.

The initial manifest format may use TOML, provided its schema is explicitly versioned.

## 6.6. Dependency resolution

The resolver should construct a graph of package identities and version constraints.

It must detect:

- Unsatisfied version requirements.
- Missing source revisions.
- Invalid manifests.
- Unresolvable package identities.
- Unsupported package metadata.
- Dependency cycles where the package model prohibits them.

Package dependencies and target dependencies should remain separate.

One package may expose multiple targets, and a consumer should not need to build every target exported by that package.

## 6.7. Source and binary packages

Glue should support three package delivery modes over time:

1. Source packages compiled by Glue.
2. Binary packages compatible with a selected target.
3. Hybrid packages that offer both source and binary artifacts.

Binary package selection must account for the platform, architecture, ABI, runtime requirements and toolchain compatibility where relevant.

A binary built for Linux ARM64 must not be assumed compatible with Android ARM64 merely because the CPU architecture matches.

## 6.8. Dependency cache

Downloaded packages should be cached separately from project build outputs.

A possible layout is:

```text
glue-cache/
    sources/
    packages/
    artifacts/
```

Immutable source revisions and verified content digests should identify cached package contents.

The cache must never treat package names alone as sufficient identifiers.

# 7. Lockfile and Reproducibility

Glue should maintain a lockfile recording the exact dependency resolution.

A proposed filename is:

```text
glue.lock
```

The lockfile should include:

- Package identities.
- Resolved versions.
- Immutable revisions.
- Content digests.
- Resolved transitive dependencies.
- Relevant manifest format versions.

A lockfile should be deterministic for equivalent resolved dependency graphs.

## 7.1. Resolution versus frozen builds

Normal dependency resolution may select newer compatible versions when permitted by the declared constraints.

A frozen resolution must instead use the existing lockfile and fail if the declarations cannot be satisfied by it.

For example:

```bash
glue deps resolve
glue deps resolve --locked
```

The first command may update the lockfile. The second should verify the existing resolution without silently changing it.

## 7.2. What reproducibility means

Glue should distinguish dependency reproducibility from binary reproducibility.

A lockfile can reproduce the selected dependencies, but it cannot by itself guarantee byte-identical outputs.

Binary reproducibility may also require:

- A pinned compiler and linker.
- Controlled environment variables.
- Stable build paths.
- Deterministic archive generation.
- Explicit generated inputs.
- Stable external tools.
- Control over timestamps and other sources of nondeterminism.

The architecture should make these conditions explicit rather than promising that every build is automatically bit-for-bit reproducible.

# 8. Internal Project Model

The project model is the semantic foundation of Glue.

It should be independent of the configuration syntax and compiler-specific command formats.

## 8.1. Core entities

The initial internal model should include:

**Project**

Contains project identity, global configuration, target declarations and dependency requirements.

**Package**

Contains canonical package identity, source metadata, resolved revision and exported targets.

**Target**

Contains target type, sources, properties, dependencies and output requirements.

**TargetPlatform**

Contains the intended operating system, architecture, ABI and other platform-specific constraints.

**Toolchain**

Contains the selected compiler and related tools, capabilities, SDK requirements and environment.

**BuildAction**

Represents one operation, such as compiling a source file, generating code or linking an executable.

**Artifact**

Represents a produced or consumed file and its relevant compatibility properties.

## 8.2. Semantic validation

Before build planning, Glue should validate the project model.

It should reject:

- Duplicate target identities.
- References to nonexistent targets.
- Invalid target dependencies.
- Incompatible property values.
- Missing source files that are not generated.
- Unsupported target combinations.
- Invalid package exports.

Where possible, errors should identify the source declaration responsible for the problem.

## 8.3. Normalization

Equivalent configurations should produce equivalent normalized models.

Normalization should resolve aliases, canonicalize relevant paths, establish target identities and make default values explicit where necessary.

This normalized model is the input to dependency resolution and target planning.

# 9. Build Engine

The build engine translates the resolved project model into an executable graph of build actions.

## 9.1. Build graph

A simple executable may produce the following graph:

```text
main.c ------> main.o ----\
                           \
                            ---> app
                           /
math.c ------> math.o ----/
```

Compilation actions can execute independently.

Linking must wait until all required object files and libraries are available.

## 9.2. BuildAction model

Each action should contain:

```text
BuildAction:
    id
    operation
    inputs
    outputs
    dependencies
    toolchain
    environment
    fingerprint
```

The action ID identifies an operation within a build plan.

The fingerprint identifies its effective inputs and configuration for incremental reuse.

These concepts should remain distinct.

## 9.3. Incremental compilation

Glue should avoid rebuilding actions whose inputs and relevant configuration have not changed.

The fingerprint should account for:

- Source content.
- Compiler identity and relevant version.
- Effective compiler arguments.
- Target properties.
- Dependency artifacts.
- Relevant environment inputs.
- Toolchain configuration.
- Generated inputs.

File timestamps can be used as an optimization, but should not be the sole basis for determining whether an artifact is valid.

## 9.4. Header dependency discovery

C and C++ compilation requires correct tracking of transitive headers.

Glue should use dependency-discovery mechanisms provided by the selected compiler when available.

Discovered dependencies must be recorded and incorporated into subsequent invalidation decisions.

If a compiler cannot provide reliable dependency information, Glue should require an explicit alternative mechanism or clearly documented limitations.

## 9.5. Build scheduling

The scheduler should execute actions only when their dependencies are satisfied.

Independent actions should be eligible for parallel execution.

The scheduler should support configurable concurrency and resource limits.

A failed action should prevent dependent actions from proceeding, while unrelated work already in progress may finish safely.

## 9.6. Generated sources

Generated files should be first-class graph outputs.

For example:

```glue
custom.add(protocol)
    .input("schema.idl")
    .output("generated/protocol.c")
    .command("idl-compiler")
```

The exact syntax remains open.

The engine must know the declared inputs, outputs and dependencies of the generator. Generated files should normally be written to the build directory, not the source tree.

Undeclared side effects should not be assumed to participate correctly in incremental builds.

## 9.7. Build backends

The initial implementation should execute compiler and linker commands directly.

For example:

```text
clang -c foo.c -o foo.o
clang foo.o -o foo
```

The exact invocation depends on the selected toolchain and target.

The internal project model should not depend on this particular execution strategy.

Alternative backends or integrations with established build systems can be considered later if they provide meaningful benefits.

# 10. Compiler and Toolchain Architecture

Glue should represent compilers and toolchains through capability-oriented interfaces.

The build engine describes the operation required. The toolchain adapter translates that operation into the appropriate native commands.

## 10.1. Toolchain responsibilities

A toolchain may define:

- Compiler discovery.
- Compiler identity and version.
- Supported languages.
- Supported targets.
- Compile command generation.
- Link command generation.
- Static archive generation.
- Dependency discovery.
- SDK and sysroot requirements.
- Runtime library selection.
- Artifact inspection.

Not every toolchain must support every capability.

The system should query capabilities explicitly rather than assuming that every compiler supports the same operations.

## 10.2. Initial toolchains

The initial implementations should prioritize Clang and GCC, followed by MSVC.

This provides a practical foundation for common Linux and Windows workflows while establishing the adapter architecture needed for additional compilers.

The implementation order should be guided by the platforms and workflows the project intends to support first.

## 10.3. Compile request

A compiler adapter should receive structured input:

```text
CompileRequest:
    source
    output
    language
    target
    optimization_profile
    include_directories
    definitions
    compile_options
    dependency_mode
    environment
```

The adapter translates this request into the actual compiler invocation.

The build engine should not construct command strings by concatenating arbitrary project values.

## 10.4. Link request

Linking should use a separate request model:

```text
LinkRequest:
    output
    target
    object_files
    static_libraries
    shared_libraries
    library_search_paths
    link_options
    runtime_requirements
```

This separation matters because compile and link options have different meanings and compatibility requirements.

## 10.5. Custom compilers

Glue should permit custom compiler integrations.

A custom integration should declare:

- The executable or tool invocation.
- Supported input types.
- Argument mapping.
- Expected outputs.
- Dependency-discovery behavior.
- Relevant environment requirements.
- Supported targets and capabilities.

This permits integration with specialized compilers without requiring the core engine to understand their internal command-line semantics.

## 10.6. Environment management

Build actions should receive an explicitly defined environment.

Relevant environment variables and tool paths must be available for fingerprinting where they affect outputs.

Glue should support ordinary local builds first, while leaving room for more isolated or hermetic execution environments.

A hermetic build may require filesystem restrictions, pinned tools and controlled external inputs. It should be an achievable mode rather than a mandatory prerequisite for every project.

# 11. Cross-Compilation

Cross-compilation should be a first-class capability.

Glue must distinguish:

**Host:** the system running the Glue process.

**Build platform:** the environment in which build tools execute.

**Target:** the environment for which the output artifacts are produced.

These concepts can refer to different systems.

## 11.1. Target representation

A target description should represent relevant properties such as:

```text
TargetPlatform:
    operating_system
    architecture
    abi
    object_format
    environment
    minimum_os_version
    api_level
```

Not every field applies to every platform.

The architecture should permit platform-specific extensions without making every platform conform to an artificial universal set of mandatory fields.

## 11.2. Target resolution

Given a target request, Glue should:

1. Resolve the requested platform.
2. Select a compatible toolchain.
3. Resolve the SDK and sysroot when required.
4. Determine the compiler and linker configuration.
5. Validate the required libraries.
6. Construct the build plan.
7. Verify the resulting artifacts.

A target name is not sufficient to guarantee that a valid cross-compilation environment exists.

Missing SDKs, incompatible libraries and unsupported compiler targets should produce explicit diagnostics.

## 11.3. Build dependencies and target dependencies

Cross-compilation requires a distinction between tools that execute on the build platform and artifacts intended for the target platform.

A code generator may run on Linux while producing source code that will later be compiled for Android.

Glue must model this distinction explicitly so that it does not attempt to execute target binaries on the host.

# 12. ABI and Binary Compatibility

Glue should represent ABI requirements explicitly rather than assuming that architecture alone determines compatibility.

Relevant properties include:

- CPU architecture.
- Operating system.
- Calling convention.
- Object and executable format.
- C and C++ ABI requirements.
- Runtime library requirements.
- Standard-library compatibility.
- Platform API level.
- Linkage and symbol requirements.
- Architecture-specific features.

## 12.1. Compatibility validation

Before consuming a precompiled artifact, Glue should determine whether the artifact is compatible with the selected target.

Compatibility may be established through declared metadata, artifact inspection and platform-specific rules.

The system should distinguish known compatibility from unverified compatibility.

It should not silently treat an unknown combination as valid.

## 12.2. C, C++ and Rust interoperability

C, C++ and Rust do not share a universal binary interface.

The initial architecture should support C and C++ targets, with explicit language and ABI requirements.

Rust should be added as a separate language integration.

Where independently compiled Rust components interoperate with C or C++, Glue should support explicitly declared foreign-function interfaces and the relevant C ABI requirements.

It should not assume that arbitrary Rust-native interfaces remain ABI-compatible across compiler versions.

## 12.3. Diagnostics

An ABI mismatch should produce a useful explanation.

For example:

```text
Dependency: graphics::renderer

Requested:
  Platform: Android
  Architecture: ARM64
  ABI: arm64-v8a
  API level: 26

Available artifact:
  Platform: Linux
  Architecture: ARM64

Result:
  No compatible artifact found.
```

This is an illustrative diagnostic.

The implementation should report the actual mismatched properties that it can establish.

# 13. Android Integration

Android is an important validation target for Glue's platform architecture.

Native Android builds require coordination between the NDK, compiler, target ABI, API level, sysroot and platform libraries.

Glue should integrate with the Android NDK instead of attempting to replace it.

## 13.1. Target declaration

A proposed interface is:

```glue
build.target.android()
    .abi("arm64-v8a")
    .api(26)
```

Alternatively, a convenience alias could be supported:

```glue
build.target("android-arm64")
```

The alias should resolve to a structured target configuration.

## 13.2. NDK resolution

The Android integration should determine:

- NDK installation and version.
- Compiler and linker paths.
- Target triple.
- ABI.
- API level.
- Sysroot.
- Required system libraries.
- Output artifact properties.

Glue should use the NDK's supported configuration mechanisms as the authoritative source of platform-specific compiler settings.

## 13.3. Multiple ABIs

A single project should be able to produce separate artifacts for multiple Android ABIs.

Examples include:

```text
arm64-v8a
armeabi-v7a
x86
x86_64
```

Each target must use its own compatible compilation and linking configuration.

Objects and libraries built for different ABIs must never be mixed in a single link operation.

## 13.4. Native libraries and complete applications

The initial Android integration should focus on producing native libraries and executables.

APK generation, signing and complete Android application packaging should be treated as a separate integration or later extension.

This boundary keeps the initial scope manageable while allowing Glue to participate in larger Android development workflows.

# 14. Execution Model and CLI

Glue should provide a consistent command-line interface.

The proposed commands are:

```bash
glue configure
glue build
glue run foo
glue test
glue clean
glue install
glue graph
glue explain foo
```

## 14.1. Build

`glue build` resolves the configured project and executes the actions needed to produce the selected targets.

It should support target selection and build profiles.

## 14.2. Run

`glue run foo` identifies the executable target, builds it if necessary and launches the resulting artifact.

Program arguments should be passed without reinterpretation after the command separator:

```bash
glue run foo -- --input data.txt
```

Glue should forward program output and return the program's exit status.

If the target cannot execute on the host, Glue should require an explicitly configured runner or emulator rather than assuming execution is possible.

## 14.3. Test

The initial test system should support executable-based tests.

Later versions can introduce richer discovery, structured results, test filtering and parallel test execution.

## 14.4. Clean

`glue clean` should remove generated build artifacts without deleting source files or downloaded dependencies by default.

## 14.5. Install

`glue install` should install supported artifacts into a declared destination.

Host artifacts and target artifacts must remain distinguishable.

A cross-compiled executable should not automatically be treated as a runnable host program.

## 14.6. Graph and explain

`glue graph` should display the relevant target or build-action graph.

`glue explain foo` should show the resolved configuration of a target, including dependencies, sources, compiler selection, target platform and output path.

These commands are especially useful when automatic resolution becomes complex.

# 15. Build Profiles

Glue should provide a small set of standard profiles.

**Debug:** suitable debugging information and development-friendly compilation settings.

**Release:** appropriate optimization and linking settings for the selected toolchain.

**Custom:** user-defined configurations.

Profiles should describe intent rather than hardcode compiler-specific flags into the core engine.

The selected toolchain translates a profile into concrete compiler and linker options.

Targets should also support configuration-specific properties without requiring duplicated target declarations.

# 16. Artifact Storage and Caching

Glue should distinguish dependency storage from build artifacts.

## 16.1. Dependency cache

Stores downloaded source packages and verified dependency contents.

## 16.2. Local build cache

Stores objects, generated files, executables, libraries and relevant build metadata.

## 16.3. Shared artifact cache

A future optional component may allow compatible build outputs to be reused across machines and CI environments.

Shared-cache keys must account for all relevant build inputs and toolchain properties.

Cache entries should be verified before use.

A shared cache must not allow artifacts to be reused merely because they have the same target name or output filename.

## 16.4. Cache correctness

The cache is an optimization, not the source of truth.

Glue should be able to invalidate cached outputs when an input changes and rebuild them from the declared graph.

Where an action depends on undeclared external state, the system must either model that state explicitly or document the limitations of caching the action.

# 17. Security Model

Glue will acquire external code and execute compilers, generators and custom build commands.

These operations must be treated as potentially unsafe.

## 17.1. Dependency integrity

Remote packages should be resolved to immutable revisions.

Downloaded contents should be checked against recorded integrity digests.

## 17.2. Build execution

Custom commands and project configuration can execute code. Glue should make such execution visible and provide clear diagnostics.

A lockfile establishes a reproducible dependency resolution; it does not establish that the dependencies are trustworthy.

## 17.3. Sandboxing

The architecture should permit future sandboxed build actions with controlled filesystem, environment and network access.

Sandboxing should be introduced through an explicit execution policy rather than assumed to exist in the initial implementation.

## 17.4. Credentials

Private repository credentials should be obtained through appropriate credential mechanisms and should not be embedded in lockfiles or logged command output.

## 17.5. Supply-chain verification

Package signatures, trusted registries, provenance metadata and policy enforcement may be added independently.

They should complement content integrity checks rather than be confused with them.

# 18. Diagnostics and Observability

Glue should report errors in terms of targets, packages and actions.

A useful error identifies:

- The affected target.
- The failed operation.
- The source or artifact involved.
- The selected toolchain.
- The relevant compiler output.
- A possible resolution when one can be established.

The complete native command and its output should remain accessible.

Glue should also provide a way to inspect resolved configuration and explain why a dependency, toolchain or artifact was selected.

This visibility is important because native builds frequently fail due to environmental or platform-specific requirements that cannot be hidden behind a generic error message.

# 19. Testing Strategy

Glue's architecture must be validated through automated tests.

## 19.1. Frontend tests

Test parsing, invalid syntax, duplicate declarations, invalid references and equivalent configurations.

## 19.2. Resolver tests

Test version constraints, transitive dependencies, conflicting requirements, immutable revisions and lockfile consistency.

## 19.3. Build graph tests

Test dependency ordering, parallel execution eligibility, cycle detection and invalidation after changes to inputs.

## 19.4. Toolchain tests

Test command generation, compiler failures, missing tools, dependency discovery and output validation.

## 19.5. Platform tests

Test target selection, toolchain compatibility, sysroot resolution and ABI validation.

## 19.6. End-to-end tests

Test complete projects that build, run, consume libraries and reproduce dependency resolution.

Android integration should include tests using a real supported NDK installation.

## 19.7. Regression tests

Every correctness bug in dependency tracking, cache invalidation or target resolution should result in a regression test.

Incremental build correctness is especially important because incorrect reuse can produce apparently successful but stale binaries.

# 20. Repository Architecture

A possible initial repository structure is:

```text
glue/
├── cli/
├── frontend/
│   ├── lexer/
│   ├── parser/
│   ├── ast/
│   ├── validation/
│   └── ir/
├── project/
├── packages/
│   ├── resolver/
│   ├── sources/
│   ├── manifests/
│   ├── lockfile/
│   └── cache/
├── targets/
├── toolchains/
│   ├── clang/
│   ├── gcc/
│   └── msvc/
├── build/
│   ├── graph/
│   ├── planner/
│   ├── scheduler/
│   └── executor/
├── artifacts/
├── platforms/
│   └── android/
└── tests/
```

This is a logical organization, not a requirement to create a separate package for every directory immediately.

The implementation should begin with a small number of modules and split them when their interfaces become stable.

# 21. Implementation Roadmap

The implementation should proceed in stages, with each stage producing a usable system.

## Stage 1 — Minimal build engine

Implement:

- Configuration parsing.
- Project and target models.
- C and C++ executable targets.
- Local compiler discovery.
- Basic compilation and linking.
- `build`, `run` and `clean`.
- Action dependency tracking.

**Exit criterion:** a small C project builds and runs through Glue.

## Stage 2 — Correct incremental builds

Implement:

- Build graph planning.
- Source and header dependency discovery.
- Action fingerprints.
- Output validation.
- Reliable invalidation.
- Parallel scheduling.

**Exit criterion:** unchanged builds reuse valid outputs, while relevant changes trigger the required rebuilds.

## Stage 3 — Libraries and target properties

Implement:

- Static and shared libraries.
- Object and interface targets where needed.
- Public and private requirements.
- Dependency propagation.
- Target validation.

**Exit criterion:** reusable libraries can be consumed by multiple executables without duplicated build configuration.

## Stage 4 — Package management

Implement:

- Local package imports.
- Git and GitHub providers.
- Package identity.
- Namespaces and aliases.
- Version resolution.
- Transitive dependencies.
- Lockfile generation and frozen resolution.
- Dependency caching.

**Exit criterion:** a project can reproduce its resolved dependency graph from a committed lockfile.

## Stage 5 — Toolchain and target abstraction

Implement:

- Structured target descriptions.
- Compiler capability discovery.
- Explicit toolchain selection.
- Custom compiler integrations.
- Sysroot support.
- Host/build/target separation.
- Artifact compatibility validation.

**Exit criterion:** the same project model can produce artifacts for multiple supported targets without duplicating the project definition.

## Stage 6 — Android integration

Implement:

- NDK discovery and selection.
- ABI and API-level configuration.
- Target-specific compiler and linker settings.
- Multi-ABI builds.
- Native library outputs.
- Actionable compatibility diagnostics.

**Exit criterion:** Glue builds and validates a native Android library against a supported NDK configuration.

## Stage 7 — Advanced capabilities

Introduce:

- Custom generators.
- Installation and packaging.
- Test orchestration.
- Shared artifact caching.
- Build graph inspection.
- Stronger execution isolation.
- Expanded platform integrations.

## Stage 8 — Rust integration

Introduce Rust as a distinct language integration.

The implementation must account for Cargo and Rust build conventions, generated code, build scripts, target compatibility and explicit interoperability requirements.

Rust should not be forced into a C/C++ compilation model when its own build semantics require different treatment.

# 22. Initial Acceptance Criteria

The first public implementation should satisfy measurable requirements.

**Basic project:** a single C source file can be compiled and executed using `glue run`.

**Incremental build:** repeating an unchanged build does not recompile unaffected sources.

**Dependency tracking:** changing a relevant header invalidates dependent compilation actions.

**Library support:** an executable can link a library defined in the same project.

**Package resolution:** a project can import a local dependency and consume its exported target.

**Remote packages:** a dependency can be resolved to an immutable revision and verified through the lockfile.

**Namespaces:** different package identities can expose similarly named targets without ambiguity in the project model.

**Toolchain selection:** a project can explicitly select a supported compiler.

**Cross-compilation:** a supported toolchain can generate artifacts for a target different from the host, with target requirements visible in the resolved configuration.

**Android:** a configured target can resolve a supported NDK and produce a native library for the requested ABI and API level.

**Diagnostics:** missing tools, incompatible artifacts and failed actions produce actionable errors.

These criteria define progress without requiring every planned capability to exist in the first release.

# 23. Open Design Decisions

Some choices should remain provisional until the implementation provides enough evidence to evaluate them.

## 23.1. Configuration language

The exact grammar, error-recovery behavior, multiline syntax and extension mechanisms require a dedicated language specification.

The architectural requirement is that configuration parsing remains separate from build execution.

## 23.2. Manifest and lockfile formats

TOML is a reasonable starting point, but the final schemas must be versioned and tested before becoming stable public interfaces.

## 23.3. Multiple versions of a package

Glue must decide whether incompatible versions of the same package identity may coexist in one dependency graph.

This decision affects namespace design, artifact isolation, ABI validation and transitive dependency resolution.

## 23.4. Build backend

Direct command execution is the recommended initial approach.

Alternative backends should be evaluated only when they provide meaningful advantages without compromising the project model or build correctness.

## 23.5. Binary package format

Binary distribution requires explicit compatibility metadata, integrity verification and a clear relationship between package versions and target-specific artifacts.

This should follow the stabilization of the target and toolchain models.

## 23.6. Rust integration

Rust should remain an extension until the core language and target abstractions have been validated with real C/C++ projects.

## 23.7. Sandbox and shared cache

Both capabilities should be designed around the action model but need not be part of the initial release.

# 24. Architectural Invariants

The following rules should remain stable across implementations.

1. Parsing a project does not implicitly compile it.
2. Every build action has declared inputs, outputs and dependencies.
3. Package identity is distinct from package display name.
4. Package dependencies are distinct from build-action dependencies.
5. Host, build and target platforms are distinct concepts.
6. Compiler-specific behavior belongs in toolchain integrations.
7. Artifacts are reused only when their relevant inputs and configuration remain valid.
8. Target compatibility cannot be inferred from CPU architecture alone.
9. Dependency lockfiles record exact resolution rather than only version constraints.
10. Native command output remains accessible even when Glue provides higher-level diagnostics.
11. Generated outputs are represented in the build graph.
12. Advanced platform support extends the project model rather than bypassing it.

These invariants are more important than any particular syntax choice because they determine whether Glue can evolve without accumulating architectural contradictions.

# 25. Reference Workflow

Consider a project with an executable and a reusable library.

Its configuration might be:

```glue
static_lib.add(core)
    .sources("src/core.c")
    .include.public("include")

exec.add(app)
    .sources("src/main.c")
    .depends(core)
```

A developer runs:

```bash
glue build
```

Glue parses the project and creates its normalized model.

It validates the target declarations and their references.

It resolves external dependencies if any are declared.

It selects a compatible compiler and target configuration.

It creates a build graph containing compilation actions for the source files and the actions required to produce the library and executable.

It calculates action fingerprints and identifies which outputs can be reused.

The scheduler executes the remaining actions, respecting dependencies and available concurrency.

The artifact store records validated outputs.

The developer then runs:

```bash
glue run app
```

Glue identifies the executable target, ensures its build is current, launches the artifact and returns the program's exit status.

The same project model can later be extended with a different compiler, target platform or external package without changing the fundamental workflow.

# 26. Minimal End-to-End Prototype

The first prototype should be deliberately small.

Project structure:

```text
hello/
├── gluefile
└── hello.c
```

`gluefile`:

```glue
exec.add(hello).sources("hello.c")
```

`hello.c`:

```c
#include <stdio.h>

int main(void) {
    puts("Hello from Glue");
    return 0;
}
```

The required command is:

```bash
glue run hello
```

Expected output:

```text
Hello from Glue
```

The prototype should also verify:

- Missing compiler detection.
- Compiler error reporting.
- Correct exit-code propagation.
- Reuse of unchanged outputs.
- Rebuilding after a source change.
- Separation of source files and generated outputs.
- Target-specific artifact directories.

Once this works reliably, the project has a foundation on which to build the package and platform systems.

# 27. Conclusion

Glue's long-term value lies in unifying native development workflows without pretending that compilers, operating systems and binary interfaces are interchangeable.

Its core should provide one project model, one dependency model and one build-action model, while preserving the differences between languages, compilers and platforms.

The initial implementation should focus on correct C/C++ compilation, incremental builds, library targets and direct execution. Package management should follow on top of a stable target model. Cross-compilation, custom toolchains, Android and Rust should extend that foundation through explicit interfaces.

The architecture should make advanced native development easier to configure, inspect and reproduce without requiring Glue to replace the underlying native ecosystem.

**The guiding principle is to unify the workflow, not erase the underlying complexity.**