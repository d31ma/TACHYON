# ADR 0112: A Delegate Takes On the Methods of Its Service

- Status: Accepted (steps 1 to 3 are in: the rename, the forwards in every
  language, the signature comparison in the typed languages and the dispatch
  by argument count in the others; step 4a adds the Yon proofs and the
  website section; step 4b1 refuses a Tac base where browser services cannot
  share, step 4b2 gives JavaScript, TypeScript, Go, Java and Dart Tac
  Delegates the page's shared service, and step 4b3 gives it to F#; after
  4b, `TY1333` counts a call through a Delegate's forward and Tac tests call
  each override and overload)
- Date: 2026-10-28
- Amends: [ADR 0047](0047-annotation-names-are-plain-english.md)
  (the 2026-09-28 amendment, Component is now Helper),
  [ADR 0069](0069-design-rules-are-build-errors.md) (layers) and
  [ADR 0031](0031-realms-mirror-each-other.md) (the mirrored role list)

## Context

`Helper` is a class role in `client/helpers/` and `server/helpers/`. A class
that needs a Helper gets its own new instance through `Needs`. A Helper may
need other Helpers, stores, repositories and clients, but not a service
(ADR 0069). It exists in every Tac and Yon language.

A common need has no good answer today: a class that should offer everything
a service offers, with one or two methods changed or added. Inheritance would
do it, but a service is a managed provider. Its instance is built and shared
by the context, so a subclass would be a second, unmanaged instance with its
own state. Writing a wrapper by hand means one forwarding method for every
method of the service, and the wrapper breaks each time the service gains a
method.

Kotlin solves this with class delegation (`class A(b: B) : B by b`) and Lombok
with `@Delegate` on a field: the compiler writes the forwarding methods, and
the author writes only the methods that change.

## Decision

### `Helper` is renamed `Delegate`

- The marker `Helper` is now `Delegate`: `@Delegate`, `[Delegate]`,
  `#[Delegate]`, `// @Delegate` in Go, `-- @Delegate` in Lua and `@delegate`
  in Python.
- The role folder is `delegates/` in both realms, and the class name ends in
  `Delegate` (`TaxDelegate`). This reverses the folder part of the ADR 0047
  amendment: `delegates/` is no longer refused.
- `helpers/` stays as a folder that is not a role. It keeps the programs that
  a `Gateway` command runs (`server/helpers/`) and behavior-only holders such
  as a `Checked` DTO (data transfer object). A `@Delegate` class there fails
  with `TY1310` and names `delegates/`.
- The build refuses `@Helper` with `TY1308` and names `Delegate`.
- `ty migrate fix --write` moves each `@Helper` class from `helpers/` to
  `delegates/`, renames `<Name>Helper` to `<Name>Delegate` with its `Needs`
  fields and types, and rewrites the marker. Gateway programs and other
  files in `helpers/` stay where they are.

A Delegate without a service (see below) behaves exactly as a Helper does
today. So the rename alone changes no behavior.

### A Delegate may take on the methods of one service

A Delegate names its service the way any class names what it needs: with a
`Needs` field whose type is a service. That service is the Delegate's
**base**.

```java
// server/delegates/TaxDelegate.java
@Delegate
public class TaxDelegate {
  @Needs
  private final TaxService tax;

  // Override: same name, same parameters. Replaces TaxService.rate(Region).
  public Money rate(Region region) {
    return tax.rate(region).times(2);
  }

  // Overload: same name, other parameters. TaxService.rate(Region) stays.
  public Money rate(Region region, LocalDate day) {
    return tax.rate(region).on(day);
  }
}
```

The build writes one forwarding method for every public method of
`TaxService` that the Delegate does not write itself. A class that needs
`TaxDelegate` can call every method of `TaxService` on it, plus the
Delegate's own methods.

Rules:

1. **One base.** A Delegate needs at most one service. Two service fields
   fail with `TY1373`. A Delegate with no service field has no base and
   forwards nothing. The base must be a service of the same realm; a store,
   client, converter or another Delegate as the base fails with `TY1373`.
2. **What is taken on.** Every public instance method that the base's own
   class declares. Not taken on: constructors, static methods, fields and
   properties, the lifecycle and event methods (`OnStart`, `OnStop`,
   `OnEvent`), the HTTP verb methods of a controller, and members that the
   build writes (`construct`, `startBehaviors`, `stopBehaviors`).
3. **Override.** A method of the Delegate with the same name and the same
   parameters as a base method replaces it. The return type must match
   (`TY1375`). The Delegate calls the base version through its field
   (`tax.rate(region)`), the way a subclass calls `super`.
4. **Overload.** A method of the Delegate with the same name and other
   parameters sits next to the base method. Both answer to the same name.
   How each language tells them apart is in the table below.
5. **The base keeps its own calls.** A forward goes to the shared base
   instance. When a base method calls another base method, it calls the base
   version, not the Delegate's override. This is the one place where a
   Delegate differs from a subclass, and it is what keeps the base a single
   shared instance with one state.
6. **Markers stay where they are written.** `Cached`, `Retry`, `MaxAtOnce`,
   `AllOrNothing` and the other method markers on a base method still apply
   when the call is forwarded, because the base runs it. A marker on an
   override applies to the override.
7. **A Delegate is not a subtype of its base.** A class that needs the base
   type does not get the Delegate. A class that wants the Delegate's methods
   names the Delegate.
8. **Name clashes.** A base method whose name is also a field, property or
   `Needs` field of the Delegate fails with `TY1375`.
9. **Instances.** As with Helper today, each class that needs a Delegate gets
   its own Delegate. Every Delegate of one base shares the one base instance.

### Where the forwards are written

The build writes the forwards when it copies the project into its source
snapshot. The Delegate's copy in the snapshot gets one forwarding method per
taken-on method, placed at the end of its class. Every language's build reads
the snapshot, so they all see the forwards. The design checks, coverage
(`TY2023`) and error locations read the files as written, so they never see a
forward and never point into one.

To list the base's methods, the build reads the same public method list that
coverage reads, keeps the methods at the class's own level, and leaves out
accessors and methods marked `OnStart`, `OnStop` or `OnEvent` (and Python
properties). A method name that the Delegate declares is not forwarded; step 3
compares parameters as well.

The dynamic languages forward every argument on unchanged:

| Language | Forward |
| --- | --- |
| JavaScript | `name(...args) { return this.field.name(...args); }` |
| TypeScript | the same, typed with `Parameters<Base['name']>` and `ReturnType<Base['name']>` |
| Python | `def name(self, *args, **kwargs): return self.field.name(*args, **kwargs)` |
| PHP | `public function name(...$args) { return $this->field->name(...$args); }` |
| Ruby | `def name(...) field.name(...) end`, after a `public` line so a `private` section above does not hide it |
| Lua | `function Delegate:name(...) return self.field:name(...) end`, after the file's last line |

The typed languages copy each base signature. The copy drops the markers
(`@CacheKey`, `[CacheKey]`, `#[...]`, `[[...]]`, `[<...>]`) and the words that
belong to the base's body (`override`, `virtual`, `abstract`, `open`, `sealed`,
and `async` in C# and Dart, whose forward returns the task or future as it
is). In Java, Groovy, Kotlin, C#, C++ and Swift each overload of a name gets
its own forward. Private overloads are not copied. In Rust a Tac Delegate
takes on only `pub` methods, because a Tac service may be compiled as its own
module; a Yon Delegate also takes on methods without `pub`, because a Yon
controller and its roles share one module. Rust functions without a `self`
receiver, such as `new`, are not taken on.

| Language | Forward | Placed |
| --- | --- | --- |
| Java, Groovy, C#, Dart | `signature { return field.name(a, b); }`, without `return` for `void` | before the class's closing brace |
| C++ | the same, with `->` for a pointer field | after a `public:` line before the class's closing brace |
| Kotlin | `signature = field.name(a, *rest)` | before the class's closing brace |
| Swift | `signature { return try await field.name(a, label: b) }`, with `try` and `await` only when the base throws or is async | before the class's closing brace |
| F# | `member this.Name(a: T) : R = field.Name(a)`, or `this.Field.Name(a)` for a `member val` field | after the type's last line |
| Go | `func (delegate *Delegate) Name(a T) R { return delegate.field.Name(a) }`, with `rest...` for a variadic parameter | after the file's last line |
| Rust | `impl Delegate { fn name(&self, a: T) -> R { self.field.name(a) } }`, with `pub` when the base method has it and `.await` for `async fn` | after the file's last line |

The arguments keep what the call needs in each language: Swift argument
labels (`_` drops the label), `&` for `inout`, C# `ref` and `out`, Dart named
parameters as `name: name`, Kotlin `*` for `vararg`, Go `...` for a variadic
parameter.

### What "same parameters" means in each language

| Languages | Same parameters means | Overload |
| --- | --- | --- |
| Java, Groovy, Kotlin, C#, F#, C++, Swift | the same parameter types in order (Swift: and the same argument labels) | native overloading; the build writes forwards only for the base signatures the Delegate does not declare |
| TypeScript | the same number of parameters (required and optional) | the build writes overload signatures for the type checker and one body that picks a method by the number of arguments |
| JavaScript, Python, Ruby, Lua, PHP | the same range of argument counts (required to most, a rest parameter counts as no upper limit); Python also matches keyword names | the build writes one method that picks a method by the arguments it gets |
| Go, Rust, Dart | the same name (these languages allow one method per name) | refused with `TY1374`, because these languages have no overloading. The help offers another name, or a call to the base field |

In the languages that pick by argument count, two methods whose ranges of
argument counts overlap but differ fail with `TY1374`, because a call could
match both. A typed method with the same parameters as a base method but
another return type is an override with another return type, and fails with
`TY1375`.

In the languages that pick by argument count, an overload is written like
this. The build renames the Delegate's own method to `_own_<name>` in the
snapshot copy and writes one method `<name>` that calls it when the number of
arguments is in its range, and the base otherwise. JavaScript and PHP count
the arguments, Python and Ruby count positional and keyword arguments
together, Lua counts `select('#', ...)`. TypeScript keeps two overload
signatures for the type checker, typed from the base and from the renamed
method. A parameter with a default value (or `?` in TypeScript, `key: value`
in Ruby) is optional, and a rest parameter (`...args`, `*args`, `**kwargs`,
`*rest`, `...` in Lua) has no upper limit. Python `self` and `cls`, a
TypeScript `this` parameter and a Ruby block are not counted.

To compare parameters, the build reads each parameter's type as written,
without its name, default value or markers: the text after the colon in
Kotlin, F#, Rust and TypeScript, the argument label and the type in Swift,
and the text before the name in Java, Groovy, C#, C++ and Dart (C# keeps
`ref`, `out` and `params`, Kotlin keeps `vararg`, C++ also compares a
trailing `const`). Go types written once for several names count for each
name. The Rust receiver (`&self`) is not compared. Types are compared as
text, so `List<String>` and `java.util.List<String>` count as different.

### Layers (ADR 0069)

- A Delegate may need its one base service, other Delegates, stores,
  repositories and clients.
- Services, controllers and Delegates may need a Delegate.
- A loop such as `TaxService -> TaxDelegate -> TaxService` still fails with
  `TY1327`.

A Delegate with a base sits between two services in the layer order, so the
order becomes: view or controller, then service, then Delegate, then service
(the base) or store or client.

### Tests

Every forwarding method is written by the build, so it has no authored logic
and is exempt from `TY2023`, as a declared HTTP request is. Every override
and overload is authored, so it needs a test. A `Fake` of a Delegate fakes
the whole Delegate, including the methods it takes on.

### Errors

| Code | When |
| --- | --- |
| `TY1308` | `@Helper` is used (names `Delegate`) |
| `TY1310` | a `@Delegate` class is outside `delegates/`, or a class there does not end in `Delegate` |
| `TY1373` | a Delegate needs more than one service, or its base is not a service of its realm |
| `TY1374` | an overload the language cannot tell apart: Go, Rust and Dart; overlapping argument counts |
| `TY1375` | an override with another return type, or a base method name that is also a field or property of the Delegate |

The numbers are checked against `develop` again before merge.

## Consequences

- The marker matrix replaces its `Helper` rows (Tac and Yon) with
  `Delegate` rows, and adds two rows, `Delegate override` and
  `Delegate overload`, so that each behavior has its own proof. The overload
  row is `refused` for Go, Rust and Dart, with `TY1374` as its proof.
- Every Helper proof is earned again under the new name, because the rename
  changes the marker, the folder and the suffix that the proofs name.
- The website's language-coverage sections that show Helper get Delegate
  tabs in every language, plus one section on base, override and overload.
- `docs/BEHAVIOR_ANNOTATIONS.md`, `docs/BEHAVIOR_PROJECT_STRUCTURE.md`,
  `docs/YON_ANNOTATIONS.md`, the ADR 0047 and ADR 0069 texts, `AGENTS.md`
  and `CLAUDE.md` change with it, and Graphify is updated.
- Projects keep working after `ty migrate fix --write`. Without it, the build
  names each `@Helper` and each `helpers/` role class.

## Alternatives considered

- **Delegate as a field marker** (Lombok's form): `@Delegate` on a field in
  any service or controller. It is more flexible, but it leaves no role for
  what Helper does today, and a class with several such fields needs rules
  for methods that two bases share.
- **Real inheritance** (`extends TaxService`): the base methods would see the
  overrides, but the subclass would be a second instance of the service with
  its own state, and Go, Rust and Lua have no class inheritance.
- **Several bases**: left out. A later ADR can allow it, with Kotlin's rule
  that a method two bases share must be overridden.

## Plan

Each step is edited in the cloud and gated on the Mac through Remote Control,
then merged to `develop` by fast-forward.

1. **Rename.** `Helper` to `Delegate` in every language and both realms,
   `delegates/` as the role folder, `TY1308` and `TY1310` refusals, `ty
   migrate fix`, docs, website, Graphify, marker matrix rows. No new behavior.
2. **Base and forwards.** Find the base, list the base's public methods with
   their parameters (extending the method lists that `TY2023` and `TY1329`
   already read), and write the forwards in every language. `TY1373`.
   Done in two parts: first JavaScript, TypeScript, Python, PHP and Ruby,
   which forward every argument unchanged; then Lua, which forwards `...`
   the same way, and the typed languages, which copy each base signature.
3. **Override and overload.** Done in two parts: first the signature
   comparison in the typed languages, `TY1374` for Go, Rust and Dart, and
   `TY1375`; then the dispatch bodies for TypeScript, JavaScript, Python,
   Ruby, Lua and PHP, with `TY1374` for overlapping argument counts.
4. **Proofs.** One Tac and one Yon example per language for base, override
   and overload (or its refusal), the new matrix rows, and a fresh marker
   matrix iteration. The Java, C# and F# Tac builds write a Delegate's `Needs`
   fields as `new` objects today, so a base service there would be a second
   instance; this step makes them pass the shared service instance to the
   base field, and the proofs show one shared state. Done in two parts:
   4a is the website section "Take on a service: override and overload" in
   the 15 Yon languages, which the published-section tests serve and call,
   with the `TY1374` refusal for Go, Rust and Dart and the error-code rows;
   4b is the Tac shared base instance and the Tac proofs. Kotlin, Swift,
   C#, Rust (browser), Ruby and Lua browser services cannot need another
   service (ADR 0065), so a Tac Delegate there has no shared service for its
   base field; Samuel chose (2026-10-28) that those languages refuse a Tac
   Delegate's base service with `TY1406` at the field (4b1), and that the
   languages whose browser services share fill the base with the page's
   instance. Python and C++ already did. 4b2 does it for JavaScript and
   TypeScript (the factory leaves an unnamed Delegate field empty and the
   page fills each graph member's field named after a page instance), Go
   (one `// @Needs` base field per Delegate, set by the holding service's
   factory from `tac.Need` after the Delegate is made) and Java (the base
   field starts with `TacNeeds.need`, and the page shares that instance),
   and Dart (the base field becomes `late` and leaves the Delegate's
   constructor, and the holding service gets an injection named after the
   page's instance that sets it). 4b3 does it for F#, where the Delegate is
   a private property of its service that the page cannot reach: an F#
   Delegate's `[<Needs>]` property of a service type is its base, the build
   starts it with `TacDelegateBase.Need<TaxService>("tax")`, and the page
   sets that hook to its own `Tac.Need` when it loads, so the Delegate gets
   the page's instance when its service makes it. The base counts as a need
   of the service that holds the Delegate, so it is shared and checked for
   circles like any other F# need, and a base that is not a page service
   fails with `TY1406`.
   After 4b (iteration 12, DL2): the visibility check (`TY1333`, ADR 0071)
   counts a call that reaches a base method through a Delegate's forward,
   so `this.taxDelegate.total(3)` keeps `TaxService.total` public and the
   Tac tests run with the check on. Tac tests compile and call an override
   in Dart, Python, C++ and native Rust, and an overload in JavaScript,
   TypeScript and Python (by argument count) and Java, C++ and F# (by
   signature); a unit test covers the Rust browser `TY1406` refusal.
