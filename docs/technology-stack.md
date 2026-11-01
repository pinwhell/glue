# Glue
## Technology Stack, Infrastructure and Reuse Strategy

**Documento:** Especificación tecnológica independiente  
**Versión:** 1.0  
**Estado:** Propuesta técnica para evaluación e implementación  
**Ámbito:** Lenguajes, bibliotecas, compiladores, backends, gestión de paquetes, plataformas, almacenamiento, herramientas de desarrollo y estrategia de reutilización.

## Resumen

Glue es un sistema de desarrollo nativo que busca unificar la definición de proyectos, la gestión de dependencias, la compilación, la configuración de toolchains, la compilación cruzada y la ejecución de aplicaciones C y C++, con posibilidad de incorporar otros lenguajes posteriormente.

El objetivo de este documento es definir una estrategia tecnológica concreta para implementar Glue utilizando infraestructura existente siempre que sea posible.

La propuesta se fundamenta en una separación clara entre tres responsabilidades:

1. Glue define la semántica de los proyectos, sus paquetes, sus targets y sus requisitos de plataforma.
2. Las herramientas especializadas existentes ejecutan las operaciones que ya resuelven eficazmente.
3. Los adaptadores de Glue conectan ambos mundos mediante interfaces internas estables.

Se propone Rust como lenguaje de implementación del núcleo, un lenguaje declarativo propio para los usuarios, Ninja como backend inicial de ejecución del grafo de compilación y Clang, GCC y MSVC como compiladores externos. La adquisición de dependencias utilizará Git y el sistema de paquetes se diseñará tomando como referencias Cargo, Conan y vcpkg.

El principio rector es evitar tanto la duplicación innecesaria de infraestructura madura como la dependencia irreversible de una herramienta externa.

---

# 1. Objetivos tecnológicos

El stack debe permitir construir un sistema que pueda evolucionar desde una herramienta de compilación pequeña hasta una plataforma integrada para desarrollo nativo.

Los objetivos prioritarios son:

- Distribuir Glue como una herramienta nativa.
- Mantener un núcleo independiente de los compiladores concretos.
- Admitir diferentes sistemas operativos, arquitecturas, ABI y toolchains.
- Separar la sintaxis de Glue de su implementación interna.
- Gestionar dependencias locales y remotas de forma reproducible.
- Aprovechar compiladores, linkers y SDK existentes.
- Facilitar la compilación incremental y paralela.
- Mantener interfaces internas sustituibles.
- Permitir incorporar nuevos lenguajes y plataformas sin rediseñar el sistema completo.
- Evitar la proliferación prematura de dependencias, servicios y formatos propietarios.

Glue no debe intentar reemplazar toda la infraestructura nativa. Debe proporcionar una experiencia coherente sobre ella.

## 1.1. Principios de selección

Toda dependencia tecnológica se evaluará mediante seis criterios:

**Madurez.** La tecnología debe resolver un problema conocido y tener documentación suficiente.

**Compatibilidad.** Debe funcionar con las plataformas que Glue pretende soportar.

**Coste de integración.** Debe reducir el trabajo total, no trasladarlo a una capa de adaptadores excesivamente compleja.

**Sustituibilidad.** Su uso no debe contaminar el modelo central del producto.

**Distribución.** Debe ser viable distribuir Glue sin introducir requisitos operativos innecesarios.

**Licencia y mantenimiento.** Las licencias, la actividad del proyecto y las condiciones de redistribución deben verificarse antes de adoptar cada componente.

Las versiones concretas de las dependencias se fijarán durante la implementación mediante pruebas de compatibilidad y una política de actualización controlada.

---

# 2. Lenguaje de implementación: Rust

## 2.1. Decisión

Rust será el lenguaje principal del núcleo de Glue.

El núcleo deberá manipular proyectos, estructuras sintácticas, grafos de dependencias, procesos externos, archivos, cachés y configuraciones de compilación. Rust proporciona seguridad de memoria, concurrencia controlada, tipos expresivos y un ecosistema adecuado para desarrollar herramientas de sistemas.

La elección no se basa en que Rust sea necesariamente superior para cualquier proyecto. Se basa en que resulta una combinación apropiada para un programa nativo que coordina múltiples herramientas y debe mantener un modelo interno complejo durante muchos años.

Referencia oficial:

https://www.rust-lang.org/

## 2.2. Responsabilidades de Rust

Rust se utilizará para:

- CLI.
- Parser y frontend del lenguaje.
- Validación semántica.
- Modelo normalizado de proyectos.
- Identidad de paquetes y targets.
- Resolución de dependencias.
- Construcción del grafo de compilación.
- Planificación de acciones.
- Adaptadores de toolchains.
- Gestión de caché y artefactos.
- Diagnósticos.
- Integración con sistemas operativos.
- Pruebas automatizadas.

Los compiladores de C y C++ no se reimplementarán en Rust.

Tampoco se impondrá Rust a los paquetes que utilizan Glue. Un proyecto administrado por Glue podrá contener C, C++, cabeceras, recursos, código generado y otros lenguajes incorporados mediante adaptadores.

## 2.3. Alternativas evaluadas

**Go** constituye una alternativa válida por su sencillez operativa, concurrencia y facilidad de distribución. Sería especialmente interesante si el proyecto priorizara una implementación rápida de un coordinador de procesos.

**C++** ofrece interoperabilidad natural con parte del ecosistema nativo, pero no elimina la necesidad de diseñar una arquitectura de módulos, resolver dependencias y gestionar la seguridad de memoria.

**Zig** merece seguimiento por su orientación a sistemas y compilación cruzada, pero no se adoptará sin comprobar la madurez del ecosistema requerido.

**Python** puede utilizarse para scripts de desarrollo, generación de fixtures y automatización auxiliar. No será la implementación principal del núcleo.

**TypeScript** podrá utilizarse en herramientas web futuras, pero no se introducirá en el motor nativo sin una necesidad concreta.

La decisión inicial es utilizar un único lenguaje para el núcleo y evitar una arquitectura poliglota prematura.

---

# 3. Organización del proyecto Rust

## 3.1. Estructura propuesta

El repositorio comenzará como un workspace de Cargo.

```text
glue/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── crates/
│   ├── glue-cli/
│   ├── glue-config/
│   ├── glue-model/
│   ├── glue-packages/
│   ├── glue-resolver/
│   ├── glue-toolchains/
│   ├── glue-build/
│   ├── glue-backend-ninja/
│   └── glue-platform-android/
├── tests/
│   ├── parser/
│   ├── resolver/
│   ├── build/
│   └── integration/
├── examples/
│   ├── hello-c/
│   ├── hello-cpp/
│   └── static-library/
└── docs/
    ├── technology/
    ├── language/
    ├── package-format/
    └── toolchains/
```

Esta estructura es el destino arquitectónico deseado, no un requisito para la primera semana de implementación.

El proyecto comenzará con pocos crates. Las fronteras se convertirán en crates independientes cuando sus contratos estén suficientemente definidos.

## 3.2. Responsabilidades de los módulos

`glue-cli` administra los comandos y comunica los resultados al usuario.

`glue-config` interpreta los archivos de configuración.

`glue-model` define los tipos centrales: proyectos, targets, dependencias, plataformas y propiedades.

`glue-packages` representa y adquiere paquetes.

`glue-resolver` resuelve versiones, identidades y restricciones de compatibilidad.

`glue-toolchains` descubre herramientas y traduce solicitudes genéricas a configuraciones concretas.

`glue-build` genera el grafo de acciones y calcula qué necesita ejecutarse.

`glue-backend-ninja` traduce el grafo al formato de Ninja.

`glue-platform-android` contiene la integración específica con Android y su NDK.

La dirección de dependencias entre módulos debe ser explícita. El modelo central no dependerá de Ninja, del parser concreto ni de las estructuras internas de una biblioteca externa.

---

# 4. Infraestructura de la CLI

## 4.1. Biblioteca seleccionada: Clap

Se propone utilizar Clap para construir la interfaz de línea de comandos.

Documentación:

https://docs.rs/clap/latest/clap/

Clap ofrece mecanismos para declarar argumentos, subcomandos, valores permitidos, opciones obligatorias y mensajes de ayuda.

Glue evitará implementar manualmente estas funciones.

## 4.2. Interfaz inicial

```text
glue init
glue configure
glue build
glue run
glue test
glue clean
glue install
glue graph
glue explain
glue package
```

Los comandos se introducirán de forma progresiva.

La primera versión solo necesita:

```text
glue init
glue build
glue run
glue clean
```

El resto podrá incorporarse a medida que exista una implementación real detrás de cada comando.

## 4.3. Contratos de salida

La CLI debe distinguir entre:

- Mensajes dirigidos a personas.
- Errores de configuración.
- Errores de compilación.
- Errores de resolución.
- Información estructurada para herramientas externas.

Las herramientas que integren Glue podrán solicitar resultados JSON cuando corresponda.

Los códigos de salida serán estables y documentados para facilitar la automatización.

---

# 5. Lenguaje de configuración de Glue

## 5.1. Decisión

Glue tendrá un lenguaje declarativo propio.

El lenguaje debe describir lo que el proyecto contiene y cómo se relacionan sus componentes. No debe obligar al usuario a escribir manualmente todos los comandos del compilador.

Ejemplo conceptual:

```glue
import json from github@alice/json version "^2.0"

static_lib.add(core)
    .sources("src/core.c")
    .include.public("include")

exec.add(app)
    .sources("src/main.cpp")
    .depends(core)
    .depends(json)
```

Este ejemplo ilustra la experiencia de usuario deseada. La gramática y las reglas de resolución deberán definirse formalmente antes de considerarlo sintaxis normativa.

## 5.2. Separación entre sintaxis y semántica

El procesamiento seguirá estas etapas:

```text
gluefile
    |
    v
Lexical analysis
    |
    v
Parsing
    |
    v
Abstract Syntax Tree
    |
    v
Semantic validation
    |
    v
Normalized Project Model
```

El AST representa lo escrito por el usuario.

El modelo normalizado representa el significado del proyecto después de resolver referencias y validar restricciones.

Esta separación permitirá evolucionar la sintaxis sin reescribir el motor de compilación.

## 5.3. Parser: evaluación de Chumsky y Pest

Se evaluarán dos bibliotecas.

**Chumsky**

https://docs.rs/chumsky/latest/chumsky/guide/

Es un sistema de combinadores de parser para Rust. Permite construir la gramática mediante código y proporciona mecanismos para gestionar errores.

**Pest**

https://pest.rs/book/

Permite describir una gramática mediante una notación PEG y mantener separadas la gramática y la lógica que procesa sus resultados.

La elección se realizará mediante un prototipo que implemente:

- Declaración de targets.
- Listas de fuentes.
- Dependencias.
- Bloques anidados.
- Errores de sintaxis.
- Ubicación precisa de errores.
- Recuperación ante errores.
- Mensajes de diagnóstico legibles.

La decisión inicial será Pest si la gramática puede mantenerse sencilla y declarativa. Si las necesidades de composición, recuperación y diagnósticos favorecen claramente un enfoque programático, se seleccionará Chumsky.

No se mantendrán dos parsers en producción.

## 5.4. Tree-sitter para herramientas de desarrollo

Referencia:

https://tree-sitter.github.io/tree-sitter/

Tree-sitter podrá añadirse cuando Glue necesite integración avanzada con editores.

Sus posibles responsabilidades incluyen:

- Resaltado sintáctico.
- Navegación por símbolos.
- Análisis incremental.
- Selección de nodos sintácticos.
- Integración con editores y servidores de lenguaje.

No es obligatorio para construir el motor de compilación inicial.

El parser del editor y el frontend semántico de Glue son componentes diferentes y pueden compartir la especificación de la gramática sin compartir necesariamente la implementación.

---

# 6. Serialización y formatos

## 6.1. Serde

Referencia:

https://docs.rs/serde/latest/serde/

Serde se utilizará para serializar y deserializar estructuras tipadas de Rust cuando sea apropiado.

No será el modelo de datos de Glue por sí mismo. El modelo estará definido por tipos propios; Serde será una herramienta para transformarlos en representaciones externas.

## 6.2. TOML

Referencia:

https://toml.io/

TOML será el formato inicial de los manifiestos editables por personas.

Por ejemplo, un manifiesto podría contener:

```toml
[package]
name = "example"
version = "0.1.0"

[[target]]
name = "app"
kind = "executable"

[target.sources]
files = ["src/main.c"]
```

El esquema es ilustrativo y todavía debe definirse formalmente.

El manifiesto describirá la identidad del paquete, sus targets y sus propiedades declarativas. La gramática del lenguaje de Glue y el formato del manifiesto deberán tener responsabilidades claramente diferenciadas.

## 6.3. Lockfile

Glue dispondrá de un lockfile para registrar la resolución concreta de dependencias.

El lockfile deberá permitir reproducir una selección de paquetes sin volver a resolver las restricciones originales en cada ejecución.

Debe registrar la información necesaria para identificar:

- El origen de cada paquete.
- La versión seleccionada.
- La revisión inmutable cuando proceda.
- La integridad de los contenidos.
- Las relaciones de dependencia.
- La información adicional que requiera el esquema de resolución.

La serialización definitiva se seleccionará después de especificar estos requisitos.

El lockfile tendrá una versión de esquema explícita y reglas de migración.

## 6.4. JSON

JSON se utilizará para interfaces estructuradas destinadas a herramientas externas.

Un comando como:

```text
glue graph --format json
```

podría proporcionar información sobre targets, acciones y dependencias.

La representación JSON no sustituirá el modelo interno ni se utilizará como formato universal para todos los archivos del sistema.

---

# 7. Gestión de versiones y hashing

## 7.1. Versiones semánticas

Se evaluará la biblioteca `semver` de Rust.

Referencia:

https://docs.rs/semver/latest/semver/

El sistema debe poder interpretar restricciones de versión y comparar versiones publicadas.

Sin embargo, comparar versiones es una operación distinta de resolver un grafo completo de dependencias.

Glue tendrá que definir explícitamente:

- Qué significa cada operador de restricción.
- Cómo se seleccionan versiones compatibles.
- Cómo se comportan las dependencias transitivas.
- Cómo se gestionan versiones prerelease.
- Qué ocurre cuando no existe una solución válida.
- Qué información se registra en el lockfile.

La biblioteca resolverá las operaciones de semántica de versiones que soporte; las políticas particulares de Glue deberán implementarse y probarse.

## 7.2. Hashing

Se propone evaluar `sha2`.

Referencia:

https://docs.rs/sha2/latest/sha2/

El hashing podrá utilizarse para verificar contenidos y construir fingerprints de acciones.

No obstante, un hash criptográfico no basta para garantizar que una compilación sea reproducible.

La clave de caché deberá tener en cuenta todos los factores que afectan al resultado, como:

- Contenido de los archivos de entrada.
- Argumentos del compilador.
- Identidad y versión de la toolchain.
- Variables de entorno relevantes.
- Target y ABI.
- Opciones de compilación.
- Dependencias declaradas.
- Generadores y herramientas auxiliares.

Los archivos temporales, las rutas absolutas y otras fuentes de variación deben tratarse según una política explícita.

---

# 8. Motor de compilación: Ninja

## 8.1. Decisión inicial

Ninja será el backend inicial de ejecución del grafo.

Documentación:

https://ninja-build.org/manual.html

Ninja es un motor de construcción orientado a ejecutar eficientemente las acciones definidas por un grafo de dependencias.

Glue no delegará en Ninja la semántica completa del proyecto. Será Glue quien decida qué targets existen, qué dependencias tienen y qué acciones hacen falta.

Ninja ejecutará una representación de ese plan.

## 8.2. Flujo de ejecución

```text
gluefile
    |
    v
Glue frontend
    |
    v
Normalized Project Model
    |
    v
Dependency resolution
    |
    v
Glue build planner
    |
    v
Build graph
    |
    v
Ninja backend
    |
    v
build.ninja
    |
    v
Ninja executor
    |
    v
Compiler and linker processes
    |
    v
Artifacts
```

## 8.3. Beneficios

Esta decisión permite aprovechar una herramienta existente para:

- Ejecutar acciones en paralelo.
- Determinar qué operaciones necesitan ejecutarse.
- Seguir relaciones entre archivos y artefactos.
- Evitar trabajo innecesario cuando las dependencias están correctamente declaradas.
- Mantener una integración relativamente sencilla con los compiladores.

Glue deberá generar reglas que representen correctamente las dependencias, los archivos producidos y los resultados esperados.

## 8.4. Limitaciones y frontera arquitectónica

Ninja no será considerado responsable de resolver paquetes, interpretar el lenguaje de Glue ni decidir la compatibilidad entre targets.

Estas funciones permanecerán en Glue.

La integración se implementará mediante un módulo separado para evitar que el modelo interno dependa directamente del formato Ninja.

## 8.5. Backend propio en el futuro

Un backend propio será una opción futura, no una meta obligatoria.

Se evaluará únicamente si aparecen requisitos medibles que justifiquen la implementación y el mantenimiento de un scheduler y ejecutor propios.

Entre las posibles razones estarían:

- Planificación distribuida.
- Ejecución remota con un modelo de seguridad específico.
- Integración profunda con un servicio de caché remota.
- Necesidades de coordinación que no puedan resolverse adecuadamente con el backend existente.

La posibilidad de sustituir el backend se conservará desde el principio.

---

# 9. Compiladores, linkers y herramientas binarias

## 9.1. Compiladores

Glue integrará inicialmente:

- Clang.
- GCC.
- MSVC.

Referencias:

https://clang.llvm.org/

https://gcc.gnu.org/

https://learn.microsoft.com/en-us/cpp/build/reference/compiler-options

Los compiladores se ejecutarán como procesos externos.

Glue será responsable de preparar los argumentos, establecer el entorno, identificar los archivos de entrada y salida y comunicar los errores.

No implementará los frontends de C o C++, ni sus optimizadores, generadores de código o mecanismos internos de compilación.

## 9.2. Linkers

La selección del linker dependerá de la toolchain y de la plataforma.

LLD será una opción cuando resulte compatible con el compilador y el entorno de destino.

Referencia:

https://lld.llvm.org/

Glue no asumirá que todos los linkers admiten las mismas opciones. Cada adaptador deberá representar las capacidades de su toolchain.

## 9.3. Abstracción de compilación

El motor utilizará una representación interna independiente del comando final.

Conceptualmente:

```text
CompileRequest
  sources
  include_paths
  defines
  language_standard
  optimization
  debug_information
  target
  toolchain
  output
```

El adaptador de Clang transformará esta solicitud en argumentos de Clang.

El adaptador de GCC hará la traducción correspondiente.

El adaptador de MSVC utilizará las opciones apropiadas de esa toolchain.

Los campos exactos se definirán a partir de las necesidades de los proyectos reales y de las diferencias entre plataformas.

La abstracción no intentará ocultar diferencias imposibles de eliminar. Las capacidades particulares podrán exponerse explícitamente mediante propiedades tipadas y mecanismos de extensión.

## 9.4. Herramientas de inspección

Glue podrá utilizar herramientas existentes para inspeccionar archivos binarios, bibliotecas y símbolos.

No se construirá inicialmente un parser universal de todos los formatos ejecutables.

La inspección de binarios se incorporará cuando una funcionalidad concreta la requiera, por ejemplo, para validar arquitecturas de una biblioteca o diagnosticar incompatibilidades de enlace.

---

# 10. Toolchains y compilación cruzada

## 10.1. Modelo conceptual

Glue debe distinguir entre el entorno en el que se ejecuta una herramienta y el entorno para el que se produce el artefacto.

Como mínimo, se modelarán:

- Build: entorno en el que se ejecuta el proceso de construcción.
- Host: entorno en el que se ejecutan las herramientas producidas, cuando corresponda.
- Target: entorno para el que se compila el artefacto final.

Esta distinción resulta especialmente importante para compiladores cruzados y generadores de código.

## 10.2. Configuración de targets

El target no se representará únicamente mediante el nombre de una CPU.

Su modelo incluirá, según corresponda:

- Arquitectura.
- Sistema operativo.
- ABI.
- Formato de objeto.
- Entorno de ejecución.
- SDK.
- Sysroot.
- Versión mínima del sistema.
- Nivel de API.
- Restricciones de compatibilidad.

Referencia de Clang:

https://clang.llvm.org/docs/CrossCompilation.html

Referencia de CMake sobre toolchains:

https://cmake.org/cmake/help/latest/manual/cmake-toolchains.7.html

Estas referencias sirven para estudiar problemas reales de configuración y compatibilidad; Glue definirá su propio modelo tipado.

## 10.3. Adaptadores

Cada toolchain tendrá un adaptador encargado de:

1. Descubrir o validar la instalación.
2. Identificar las herramientas disponibles.
3. Comprobar sus capacidades.
4. Resolver rutas y configuración.
5. Traducir solicitudes de compilación y enlace.
6. Identificar las propiedades relevantes para la caché.
7. Comunicar diagnósticos comprensibles.

Una toolchain personalizada podrá proporcionar ejecutables, rutas, argumentos y metadatos mediante una interfaz documentada.

Las extensiones no deberían necesitar modificar el núcleo para introducir un compilador nuevo.

---

# 11. Integración con Android

## 11.1. Decisión

Glue reutilizará Android NDK y sus herramientas.

Referencia:

https://developer.android.com/ndk

La integración inicial se centrará en compilar bibliotecas y ejecutables nativos para Android.

No se implementará un compilador Android propio ni se reconstruirá el SDK.

## 11.2. Configuración conceptual

```glue
build.target.android()
    .abi("arm64-v8a")
    .api(26)
```

La sintaxis es ilustrativa.

El adaptador deberá resolver la instalación del NDK, la toolchain apropiada, el target del compilador, el sysroot y las bibliotecas correspondientes.

## 11.3. ABIs

La integración deberá permitir construir para las ABIs soportadas por la versión del NDK seleccionada.

Cada configuración será una combinación explícita de plataforma, ABI y nivel de API.

Glue no asumirá que un artefacto compilado para una ABI puede utilizarse automáticamente en otra.

## 11.4. Aplicaciones completas

La generación de bibliotecas nativas y la creación de una aplicación Android completa son problemas distintos.

Para producir APK y gestionar el ciclo de vida completo de una aplicación, Glue podrá integrarse con Gradle y Android Gradle Plugin.

La integración se realizará de forma que Glue mantenga su propio modelo para la compilación nativa sin duplicar las responsabilidades del ecosistema oficial de Android.

---

# 12. Gestión de paquetes: reutilización y diseño propio

## 12.1. Problema

El ecosistema C/C++ no cuenta con un único modelo universal que cubra todos los paquetes, binarios, toolchains y configuraciones de destino.

Glue debe aprender de los sistemas existentes sin asumir que sus modelos son intercambiables.

Se estudiarán especialmente:

- Cargo.
- Conan.
- vcpkg.
- Meson Wrap.

Referencias:

https://doc.rust-lang.org/cargo/reference/

https://conan.io/

https://vcpkg.io/

https://mesonbuild.com/Wrap-dependency-system-manual.html

## 12.2. Cargo como referencia de resolución

Cargo proporciona una referencia madura para estudiar manifiestos, restricciones de versiones, resolución transitiva, lockfiles y caché.

Sin embargo, Glue no adoptará automáticamente el modelo completo de Cargo como formato universal de paquetes.

Su dominio incluye C y C++, bibliotecas precompiladas, SDK, toolchains personalizadas y restricciones de ABI que requieren un modelo más general.

Cargo podrá seguir utilizándose para proyectos Rust incorporados a Glue, con una integración explícita entre ambos sistemas.

## 12.3. Conan y vcpkg

Conan y vcpkg se estudiarán para entender cómo integrar paquetes C/C++ existentes.

Glue deberá evitar obligar a los usuarios a volver a descargar, reconstruir o publicar todos sus paquetes cuando ya existe una distribución compatible.

Se evaluarán adaptadores para:

- Consumir paquetes ya instalados.
- Importar metadatos de targets.
- Respetar la configuración de la toolchain.
- Verificar la compatibilidad binaria.
- Evitar conflictos de configuración.
- Mantener trazabilidad de las dependencias.

No se presupone que cualquier paquete de esos ecosistemas pueda importarse sin transformación. La compatibilidad deberá validarse para cada modelo.

## 12.4. Modelo propio de Glue

Glue definirá sus propias identidades de paquetes, referencias de targets y contratos de compatibilidad.

La identidad de un paquete no será equivalente a su nombre de presentación.

Tampoco se confundirá la dependencia entre paquetes con la dependencia entre acciones de compilación.

El modelo debe permitir diferenciar:

- Fuente del paquete.
- Identidad del paquete.
- Versión.
- Revisión.
- Targets exportados.
- Dependencias.
- Restricciones de plataforma.
- Propiedades de ABI.
- Artefactos disponibles.
- Integridad de los contenidos.

## 12.5. Resolución de versiones

La resolución de dependencias deberá ser determinista bajo un conjunto fijo de manifiestos, restricciones, repositorios y políticas.

El resolvedor debe detectar incompatibilidades y explicar por qué no existe una solución válida.

La política sobre múltiples versiones del mismo paquete debe establecerse formalmente. No debe depender de comportamientos accidentales del orden de descarga.

El lockfile registrará la resolución seleccionada.

La implementación del resolvedor se comparará con algoritmos y diseños existentes antes de optar por una implementación completamente propia.

---

# 13. Git y fuentes remotas

## 13.1. Decisión

La primera versión utilizará Git como proceso externo.

Referencia:

https://git-scm.com/doc

Glue no implementará su propio protocolo Git.

La adquisición de una dependencia seguirá una secuencia controlada:

```text
Dependency declaration
        |
        v
Source identification
        |
        v
Version or revision selection
        |
        v
Git acquisition
        |
        v
Revision verification
        |
        v
Content integrity checks
        |
        v
Package validation
        |
        v
Resolved package
```

La revisión final deberá quedar identificada de manera reproducible.

## 13.2. Ejemplo

```glue
import json from github@alice/json version "^2.0"
```

La declaración es una referencia legible para el usuario.

El resolvedor debe registrar la revisión concreta seleccionada, sin depender de que una rama remota conserve su contenido para siempre.

La revisión Git y el digest de integridad se tratarán como datos distintos.

## 13.3. Caché de fuentes

Las fuentes adquiridas podrán almacenarse en una caché local reutilizable.

La caché debe separar los contenidos descargados de los directorios de trabajo y de los artefactos de compilación.

El sistema deberá poder identificar cuándo un paquete está disponible localmente y cuándo necesita adquirirse de nuevo.

## 13.4. Seguridad

Los repositorios remotos constituyen una entrada no confiable.

Glue deberá validar la procedencia, evitar sobrescrituras de rutas fuera de las ubicaciones permitidas y controlar qué operaciones ejecuta al procesar un paquete.

La adquisición de fuentes no debe implicar automáticamente la ejecución de scripts arbitrarios.

Las capacidades que ejecuten código durante la configuración, generación o compilación necesitarán contratos explícitos y documentación de seguridad.

---

# 14. Namespaces, identidad y ABI

## 14.1. Namespaces de targets

Glue podrá permitir referencias como:

```text
alice_json::json
bob_json::json
```

El propósito es distinguir targets dentro del modelo de Glue, aunque sus nombres visibles coincidan.

## 14.2. Separación de identidades

El sistema diferenciará cuatro conceptos:

1. Identidad del paquete.
2. Nombre del target.
3. Identidad del artefacto.
4. Identidad de los símbolos binarios exportados.

Esta separación es esencial para evitar falsas garantías de aislamiento.

## 14.3. Compatibilidad binaria

Un namespace de Glue no modifica por sí solo los símbolos exportados por una biblioteca C o C++.

Dos bibliotecas pueden tener nombres de paquete distintos y, aun así, exportar símbolos incompatibles o entrar en conflicto durante el enlace.

Glue debe validar las propiedades de compatibilidad que pueda conocer y permitir que los adaptadores de plataforma incorporen reglas adicionales.

Cuando no pueda demostrar la compatibilidad, debe evitar presentar una combinación como garantizada.

---

# 15. Caché, almacenamiento y reproducibilidad

## 15.1. Almacenamiento inicial

La primera versión utilizará el sistema de archivos para:

- Fuentes descargadas.
- Artefactos de compilación.
- Archivos generados.
- Lockfiles.
- Metadatos locales.

SQLite se evaluará cuando el volumen de metadatos o las necesidades de consulta lo justifiquen.

Referencia:

https://www.sqlite.org/whentouse.html

No se introducirá una base de datos obligatoria antes de que exista un requisito real.

## 15.2. Fingerprints

Cada acción de compilación tendrá una identidad calculada a partir de sus entradas relevantes.

Conceptualmente:

```text
ActionFingerprint =
    hash(
        action_type,
        input_content,
        toolchain_identity,
        compiler_arguments,
        environment,
        target_configuration,
        dependency_fingerprints
    )
```

Esta expresión describe el principio del diseño, no una serialización definitiva.

El cálculo real deberá utilizar una representación canónica, versionada y sin ambigüedades.

## 15.3. Reproducibilidad

La reproducibilidad exige más que utilizar un lockfile.

Glue deberá controlar o registrar las entradas que pueden alterar el resultado, incluidas las versiones de herramientas, las variables de entorno relevantes, las dependencias y la configuración de destino.

Los artefactos se considerarán reutilizables solo cuando las reglas de validez de la caché indiquen que corresponden a las entradas actuales.

## 15.4. Caché remota

Una caché remota se considerará una evolución posterior.

Antes de introducirla, Glue deberá contar con fingerprints estables, identificación correcta de entradas, separación de artefactos y mecanismos de integridad.

La caché distribuida no debe convertirse en una condición para utilizar el sistema localmente.

---

# 16. Testing y calidad

## 16.1. Herramientas iniciales

Se utilizará `cargo test` para pruebas unitarias y de integración.

Referencia:

https://doc.rust-lang.org/cargo/commands/cargo-test.html

Las pruebas deberán validar contratos, no solo funciones aisladas.

## 16.2. Pruebas del parser

Se mantendrán fixtures para:

- Archivos válidos.
- Errores de sintaxis.
- Errores semánticos.
- Referencias desconocidas.
- Declaraciones duplicadas.
- Bloques incompletos.
- Mensajes de diagnóstico.
- Compatibilidad entre versiones del lenguaje.

## 16.3. Pruebas del resolvedor

Se probarán:

- Resoluciones válidas.
- Restricciones incompatibles.
- Dependencias transitivas.
- Dependencias circulares cuando estén prohibidas.
- Versiones prerelease.
- Fuentes locales y remotas.
- Resolución reproducible.
- Lockfiles incompletos o inválidos.
- Errores de integridad.

## 16.4. Pruebas de compilación incremental

El sistema debe demostrar que:

- Una entrada sin cambios no fuerza innecesariamente una recompilación completa.
- Modificar una fuente recompila las acciones afectadas.
- Modificar una cabecera afecta a sus consumidores.
- Los archivos generados aparecen en el grafo.
- Cambiar una propiedad relevante invalida los resultados correspondientes.
- Un fallo de compilación no deja el estado como si la operación hubiera tenido éxito.

## 16.5. Pruebas multiplataforma

Se incorporarán pruebas de compilación en los sistemas operativos prioritarios y pruebas cruzadas para targets seleccionados.

La matriz exacta se definirá según las plataformas soportadas en cada versión.

---

# 17. Integración continua y desarrollo

## 17.1. CI

GitHub Actions es una opción inicial para automatizar pruebas y builds.

Referencia:

https://github.com/features/actions

La elección definitiva del proveedor de CI no forma parte del modelo semántico de Glue.

El pipeline inicial deberá ejecutar:

- Formateo.
- Lints.
- Pruebas unitarias.
- Pruebas de integración.
- Compilación de los ejemplos.
- Validación de artefactos distribuibles.

## 17.2. Política de dependencias

El proyecto deberá mantener un inventario de dependencias directas y transitivas.

Antes de incorporar una biblioteca se revisarán:

- Licencia.
- Mantenimiento.
- Compatibilidad con los sistemas operativos.
- Compatibilidad con la versión de Rust seleccionada.
- Dependencias transitivas.
- Riesgos de seguridad.
- Coste de distribución.
- Existencia de alternativas más simples.

Las actualizaciones se realizarán de forma controlada, acompañadas de pruebas.

## 17.3. Distribución

Glue se distribuirá inicialmente como un ejecutable nativo por plataforma.

Los paquetes de distribución deberán especificar sus requisitos y las herramientas externas que el usuario necesita instalar.

No se prometerá que una única descarga contenga automáticamente todos los SDK, compiladores y toolchains posibles.

El descubrimiento de herramientas y la comunicación de requisitos formarán parte de la experiencia de instalación.

---

# 18. Matriz de decisiones tecnológicas

| Área | Decisión inicial | Alternativa o referencia | Motivo |
|---|---|---|---|
| Lenguaje del núcleo | Rust | Go, C++ | Seguridad de memoria y modelo tipado |
| CLI | Clap | Parser manual | Evitar infraestructura redundante |
| Parser | Prototipo Pest/Chumsky | Tree-sitter para editores | Selección basada en pruebas |
| Modelo de proyecto | Tipos propios de Glue | Ninguna sustitución directa | Mantener semántica independiente |
| Serialización | Serde | Implementación manual | Reutilizar infraestructura estable |
| Manifiestos | TOML | Otros formatos tras evaluación | Configuración legible |
| Versiones | `semver` | Implementación propia solo si hace falta | Reutilizar operaciones estándar |
| Integridad | SHA-256 mediante `sha2` | Algoritmo alternativo justificado | Verificación de contenidos |
| Fuentes remotas | Git CLI | Biblioteca Git si existe una necesidad | Reducir complejidad inicial |
| Build backend | Ninja | Scheduler propio futuro | Llegar antes a una versión funcional |
| Compiladores | Clang, GCC, MSVC | Otros mediante adaptadores | Aprovechar toolchains existentes |
| Android | Android NDK | Integraciones futuras | Utilizar el entorno oficial |
| Paquetes Rust | Integración con Cargo | Adaptador propio | Respetar su ecosistema |
| Paquetes C/C++ | Adaptadores de Conan/vcpkg | Registro nativo futuro | Aprovechar paquetes existentes |
| Almacenamiento | Sistema de archivos | SQLite cuando sea necesario | Evitar complejidad prematura |
| Caché remota | Fase posterior | Proveedores especializados | Requiere fingerprints estables |
| Testing | Cargo test | Herramientas auxiliares | Integración directa con Rust |
| CI | GitHub Actions inicialmente | Otro proveedor | Automatización reproducible |

---

# 19. Plan de implementación tecnológica

## Fase 1. Núcleo ejecutable

Implementar el workspace Rust, la CLI, el manifiesto mínimo y la construcción de un único ejecutable C.

Resultado: Glue puede crear, compilar y ejecutar un proyecto sencillo.

## Fase 2. Modelo de targets

Introducir targets de bibliotecas, dependencias internas, propiedades de compilación y enlace.

Resultado: proyectos con múltiples unidades de compilación y bibliotecas reutilizables.

## Fase 3. Corrección incremental

Implementar fingerprints, seguimiento de dependencias, generación de archivos y reglas de invalidación.

Resultado: reconstrucciones correctas y eficientes.

## Fase 4. Gestión de paquetes

Implementar identidad de paquetes, fuentes locales, adquisición Git, versiones, resolución transitiva y lockfile.

Resultado: proyectos reproducibles con dependencias externas.

## Fase 5. Toolchains

Introducir adaptadores configurables, separación host/build/target y validación de compatibilidad.

Resultado: capacidad de utilizar diferentes compiladores y realizar compilación cruzada.

## Fase 6. Android

Integrar el NDK, ABI, API levels y validación de bibliotecas nativas.

Resultado: compilación de targets nativos de Android.

## Fase 7. Ecosistemas externos

Incorporar adaptadores para paquetes C/C++ existentes y mejorar la integración con entornos de desarrollo.

Resultado: adopción progresiva del ecosistema nativo existente.

## Fase 8. Capacidades avanzadas

Evaluar caché remota, ejecución distribuida, integraciones con editores y un backend de ejecución propio.

Resultado: evolución guiada por necesidades verificadas, no por complejidad anticipada.

## Fase 9. Rust como lenguaje de proyecto

Integrar Cargo y sus herramientas como un subsistema especializado.

Resultado: proyectos mixtos que puedan combinar bibliotecas nativas y componentes Rust mediante contratos explícitos.

---

# 20. Riesgos tecnológicos y mitigaciones

## 20.1. Acoplamiento a Ninja

**Riesgo:** el modelo de Glue termina reflejando las limitaciones o convenciones del backend.

**Mitigación:** mantener el grafo interno y el adaptador de Ninja como componentes separados.

## 20.2. Abstracción excesiva de compiladores

**Riesgo:** una interfaz genérica intenta ocultar diferencias fundamentales entre toolchains.

**Mitigación:** representar las capacidades y diferencias de forma explícita; no reducir todas las herramientas a un mínimo común artificial.

## 20.3. Resolver de paquetes demasiado ambicioso

**Riesgo:** el proyecto invierte demasiado tiempo en un sistema universal antes de tener usuarios y casos reales.

**Mitigación:** comenzar con fuentes locales y Git, definir contratos claros y comparar algoritmos existentes antes de implementar el resolvedor completo.

## 20.4. Caché incorrecta

**Riesgo:** reutilizar un artefacto construido con entradas o herramientas diferentes.

**Mitigación:** fingerprints completos, pruebas de invalidación y políticas estrictas de validez.

## 20.5. Complejidad multiplataforma

**Riesgo:** la abstracción del target no representa correctamente las diferencias entre sistemas.

**Mitigación:** comenzar con plataformas concretas y añadir propiedades del modelo cuando los casos reales lo requieran.

## 20.6. Dependencias externas excesivas

**Riesgo:** la suma de bibliotecas aumenta los costes de mantenimiento y distribución.

**Mitigación:** inventario de dependencias, evaluación de licencias, pruebas de compatibilidad y preferencia por componentes pequeños y mantenidos.

## 20.7. Integración prematura de demasiados ecosistemas

**Riesgo:** intentar soportar simultáneamente Cargo, Conan, vcpkg, Android y múltiples toolchains retrasa el primer producto funcional.

**Mitigación:** definir primero un recorrido completo mínimo y añadir adaptadores después.

---

# 21. Criterios de aceptación del stack

La estrategia tecnológica se considerará validada cuando el prototipo pueda demostrar lo siguiente:

1. Construir y ejecutar un proyecto C sencillo.
2. Construir una biblioteca y utilizarla desde otro target.
3. Detectar cambios en las fuentes y reconstruir únicamente las acciones afectadas.
4. Generar diagnósticos útiles para errores de configuración y compilación.
5. Obtener una dependencia Git y registrar una resolución reproducible.
6. Seleccionar una toolchain sin modificar el núcleo de Glue.
7. Representar de manera explícita las diferencias entre targets.
8. Compilar un target cruzado con una toolchain compatible.
9. Integrar una biblioteca externa mediante un adaptador.
10. Mantener el modelo de proyecto independiente del backend Ninja.
11. Ejecutar pruebas automatizadas en las plataformas prioritarias.
12. Distribuir el ejecutable de Glue con requisitos documentados.

Estos criterios validan la arquitectura tecnológica, no todas las capacidades futuras del producto.

---

# 22. Decisiones que deben permanecer abiertas

Algunas decisiones requieren evidencia de implementación antes de fijarse definitivamente.

**Parser.** Seleccionar Pest o Chumsky mediante un prototipo comparativo.

**Resolvedor de paquetes.** Determinar qué componentes existentes pueden reutilizarse y cuáles necesitan una implementación específica de Glue.

**Formato definitivo del lockfile.** Fijarlo después de definir el modelo de resolución.

**Backend propio.** Mantenerlo como posibilidad, sin convertirlo en requisito de la primera versión.

**Base de datos local.** Introducir SQLite solo cuando las necesidades de consulta y almacenamiento la justifiquen.

**Integración de Rust.** Definir su alcance después de consolidar el modelo de targets y toolchains.

**Distribución multiplataforma.** Establecer la matriz oficial de plataformas y toolchains según los objetivos de lanzamiento.

Cada decisión abierta debe tener un criterio explícito de resolución. No se dejará indefinidamente sin evaluar ni se cerrará únicamente por preferencia personal.

---

# 23. Conclusión

Glue debe construir una capa unificada sobre infraestructura nativa existente, no competir con cada herramienta especializada en su propio dominio.

Rust proporciona el lenguaje de implementación. El DSL ofrece la experiencia declarativa. El modelo normalizado representa la semántica del proyecto. El resolvedor coordina paquetes y versiones. Los adaptadores traducen el modelo hacia toolchains y backends. Ninja ejecuta el grafo inicial, mientras que Clang, GCC, MSVC y las herramientas de plataforma producen los artefactos.

Git proporciona adquisición de fuentes. Cargo, Conan y vcpkg sirven como referencias e integraciones para sus respectivos ecosistemas. Android NDK permite abordar la compilación nativa para Android sin reconstruir su infraestructura.

La arquitectura debe conservar una separación estricta entre el modelo propio de Glue y los detalles de las herramientas que utiliza.

Esta separación es la principal inversión tecnológica: permite cambiar un parser, incorporar un compilador, añadir una plataforma o sustituir el backend sin rehacer el producto completo.

**La decisión central es desarrollar únicamente aquello que constituye la identidad de Glue y reutilizar infraestructura madura para todo lo demás.**

El siguiente paso es convertir este documento en un inventario verificable de dependencias, un prototipo mínimo y una serie de pruebas de aceptación. La implementación deberá confirmar las decisiones con código real, en lugar de asumir que una propuesta tecnológica es correcta únicamente porque parece coherente sobre el papel.