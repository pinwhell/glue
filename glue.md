Glue is a native development system inspired by CMake’s build model and the simplicity of package managers like npm and Composer. Its goal is to bring building, dependency management, and execution into a single, cohesive experience for C and C++, with Rust as a possible future direction.

The idea is to keep project definitions expressive and familiar while making common workflows feel more straightforward:

exec.add(foo).sources(foo.c)

Dependencies could be imported from remote repositories or local directories, with namespaces helping prevent naming conflicts. Glue would also aim to make running a target as simple as glue run foo, handling the necessary build steps when needed.

Beyond the basic workflow, the long-term vision includes flexible compiler and toolchain support, cross-compilation, and better handling of platform-specific requirements such as Android targets and ABIs.

Rather than trying to solve every aspect of native development at once, Glue would start with a small, coherent foundation and grow toward a unified experience for defining, building, managing, and running native software.