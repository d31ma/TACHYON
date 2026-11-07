# ADR 0072: Groovy Is a Yon Language

- Status: Accepted
- Date: 2026-09-28
- Builds on: [ADR 0048](0048-yon-routes-come-from-annotations.md) (routes),
  [ADR 0051](0051-controller-error-handlers.md) (error handlers),
  [ADR 0043](0043-models-hold-data-types.md) (models),
  [ADR 0056](0056-every-annotation-checks-its-target.md) (the shared rule
  table) and [ADR 0062](0062-yon-uses-native-http-types.md) (native HTTP types)
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (the Yon languages)

## Context

Many JVM teams write Groovy (Gradle scripts, Grails, Jenkins, Spock tests).
Groovy runs on the same JDK as Java, reads Java declarations, and has
annotations with the same `@Name` spelling. Tachyon already runs Java with a
Java protocol runtime, so Groovy can reuse that runtime instead of getting a
new one.

## Decision

### Realm

- Groovy (`.groovy`) is a Yon language: controllers, services and the other
  roles, error handlers, workers, root middleware (`middleware.groovy`),
  models in `server/models/` and test units.
- Groovy is not a Tac language. Groovy has no production WebAssembly or
  JavaScript target.

### Annotations and types

- A Groovy annotation is `@Name` or `@Name(arguments)`, as in Java. The
  names and the arguments are the same as in every other language. Tachyon
  reads the annotations and removes them before Groovy compiles the file.
- Tachyon reads Groovy declarations with the Java rules. A semicolon at the
  end of a line is optional.
- Every class member has a declared type: `String name`,
  `List<Order> findOrders()`. A field or method declared with `def` is a
  build error (TY1334), because Tachyon reads member types to check routes,
  models and injection. A local variable inside a method may use `def`.
- Single-quoted, double-quoted (GString) and triple-quoted strings are text
  for Tachyon. Tachyon reads slashy strings (`/.../` and `$/.../$`) as code,
  so an annotated file writes regular expressions in quoted strings.
- A model in `server/models/` is a record without a body, an enum with
  constants or a class with typed fields. A controller imports it with
  `import models.Order`.
- Groovy follows the Spring and Lombok meaning of each annotation, like Java.
  `Needs`, `NeedsAll`, the behavior annotations, error handlers and test
  units work as in Java, and so do `StreamRoute` and `InBackground` (see the
  amendment of 2026-10-17).

### Runtime

- The request and the response are `com.sun.net.httpserver.HttpExchange`, as
  in Java (ADR 0062).
- Tachyon splits the program in two parts: the author's Groovy (the composed
  role files and the models) and the generated Java (the Java protocol
  runtime, the containers, the route calls and the behavior runtime). Groovy's
  joint compilation (`FileSystemCompiler -j`) compiles both parts together,
  so each part can use the classes of the other. The generated code stays
  Java because Groovy reads some Java forms differently (array initializers
  and `$` inside double-quoted strings).
- Tachyon packs the compiled classes and the Groovy core runtime jar into one
  handler jar, as Kotlin does with `-include-runtime`, caches it by the digest
  of both parts, and runs it with `java -jar`. Groovy modules other than the
  core (for example `groovy-json`) are not in the jar; a handler uses the Java
  standard library, and models become JSON through Tachyon.
- Tachyon needs Groovy 4 or newer (the repository pins 5.1.3) and a JDK. It
  finds Groovy through `GROOVY_HOME`, or through `groovyc` on `PATH`. Startup
  readiness and `ty doctor` check for the Groovy runtime jar and for `java`.
- `ty init` offers Groovy as a server language. When Groovy is missing, it
  installs Groovy 5.1.3 with SDKMAN on macOS and Linux and shows the manual
  steps on Windows.
- Firecracker isolation does not run Groovy, the same as Java.

## Consequences

- Groovy joins the Yon examples that Java has on the website, and the
  language page lists it.
- A decimal literal such as `1.5` is a `BigDecimal` in Groovy, not a
  `double`. Code that compares numbers from JSON writes `1.5d`.
- Groovy gives a closure or lambda its interface type only where a parameter
  names that type. A `Setup` that puts an adapter into a map writes the type:
  `{ request, execution -> ... } as Behaviors.HttpAdapter`.
- Groovy reads one map argument to a record constructor as named arguments.
  Code that builds a record from a map value names the component:
  `new CatalogSummary(summary: summary)`.
- The first request after a change compiles the handler once (a few seconds
  for joint compilation); later requests reuse the cached jar.

## Amendment (2026-10-17): Groovy streams and background listeners

Groovy runs on the generated Java part, so it takes the Java stream and
listener runtime unchanged.

- A Groovy `@StreamRoute` method declares `Iterator<T>` or
  `java.util.stream.Stream<T>` as its return type, for example
  `return (2..1).collect { new Tick('ANVIL-1', it) }.iterator()`. An untyped
  `def` method or any other return type is refused with TY2015, and so is a
  route that returns a Stream or Iterator without `@StreamRoute`.
- `@InBackground` on an `@OnEvent` listener runs it after the other listeners
  of the same event, on the same thread, and never republishes its return
  value, as in Java and Yon JavaScript.
