# ADR 0057: NeedsAll Requires Its Fields

- Status: Accepted
- Date: 2026-09-26
- Amends: [ADR 0030](0030-components-use-services.md) (how a Tac service gets
  the properties a component passes) and
  [ADR 0055](0055-annotation-arguments-use-types.md) (NeedsAll takes no
  arguments)
- Preserves: [ADR 0056](0056-every-annotation-checks-its-target.md) (the rule
  table; NeedsAll is bare and a NeedsAll class has no constructor)

## Context

Tachyon annotations follow Spring and Lombok, plus Tachyon guards. Before this
decision a Tac `@NeedsAll` service accepted any property a component passed,
and a missing property left the field empty (`undefined`, `null`, `nil` or a
zero value). The service then failed later, far from the page that forgot the
property.

## Decision

One rule for Tac and Yon: `@NeedsAll` means Lombok's
`@RequiredArgsConstructor`. A field without an initial value is required, and
a field with an initial value is plain state. Tachyon fills every required
field before the class runs, never fills plain state, and never leaves a
required field `null` or `nil`. In Tac a required field is a collaborator or
a prop. In Yon it is always a collaborator, because Yon has no props.

A required field is filled from one of two places:

- a collaborator is injected: its name is a service instance name, its type is
  a service, store, client, repository, configuration, delegate, converter or
  entity class, a class in the same file, or `ApplicationEventPublisher`, or it
  carries its own marker (for example `@Needs`);
- every other required field comes from the component's props.

Which fields have no initial value, per language:

| Language | Required field |
| --- | --- |
| JavaScript, TypeScript | a field without `=` (`open`, `open!: boolean`); `label?: string` is refused (see the amendment) |
| Dart | a `final` or `late` field without `=` |
| C# | a `readonly` field without `=`, or a `required` member; the prop name is lowerCamel (`Docs` is `docs`) |
| Kotlin | a `val` without `=`, or a `lateinit var` |
| Swift | a `let` without `=`, or a non-optional `var` without `=` |
| Rust | every field, unless the struct derives or implements `Default` (then none) |
| Go | a field marked `// @Required` (Lombok's `@NonNull`); every other Go field has a zero value and is plain state |
| Ruby | every `attr_*` field; a Ruby class that needs defaults for plain state writes a plain `initialize` and no `NeedsAll` |

Rust and Ruby props use the lowerCamel page name (`page_path` is `pagePath`).

A JavaScript or TypeScript service that uses managed clients no longer refuses
a field that names no client: that field is a prop field, and the generated
constructor reads it from the component's props.

Build time: when a page or component uses a component whose prototype service
has required prop fields, the compiler refuses a usage that does not pass each
one (`TY1407`), for example "DrawerService needs `open`; pass :open="..." to
<docs-sidebar> (from <docs-layout>)". A usage that forwards a bare name
(`:open="open"`) must itself receive that name, unless the name is a loop
binding, a service of the enclosing component, `data` or `page`.
A usage that passes a property which a service of that component gets
injected, and which no service of that component requires, is refused with
`TY1407` too. A field named like a service instance (`documentation` next to
`DocumentationService`) is that service, as the list above says, so a view
that passes `:documentation` would lose the value. Spring semantics decide:
a name or a type that points at a provider is a declaration of injection,
and a prop never silently replaces it, so the build names both readings and
asks the author to rename the field or to stop passing the property. A singleton
service with required prop fields is refused, because a singleton gets no
props.

Run time: every Tac service, in every language, is created by one function of
the browser runtime (`createServiceInstance` in `tac-client.js`), before the
language glue runs. It refuses a required field whose value is missing or
`null` and does not create the service. The plan lists the required fields of
each service (`"required": [...]`).

Compiled Tac runtimes (Dart, Rust, C#, Go, Swift, Kotlin, Ruby): the plan
lists `required` for every compiled service, and `createServiceInstance`
passes only those props to the language glue. A prop never fills an
initialized field. A Rust NeedsAll struct without `Default` is built with
each field's default value and then receives its props, so it needs no
`fn new`. A C# class with `required` members is built with an object
initializer and then receives its props.

Swift and Kotlin refuse an immutable field without a value unless a
constructor fills it, and a NeedsAll class may not write one. So Tachyon
writes it, like Lombok writes `@RequiredArgsConstructor`. For a native Swift
service it adds `init?(tacProps: [String: Any])`, which sets every required
field (`let x: T`, and `var x: T` that is not optional) from the props. For a
native Kotlin service it adds a secondary constructor
`constructor(tacProps: org.json.JSONObject?)`, which sets every `val x: T`
and `lateinit var x: T`. The native glue creates the service with these
props. If a value is missing or has the wrong type, the Swift constructor
returns `nil` and the Kotlin constructor throws; the page gets
`TY_NATIVE_REQUIRED` and the service is not created. The `TY1407` build check
and the browser runtime check run first, as for every other language. In Yon,
Kotlin already gets a generated constructor that takes the injected
dependencies, and Yon has no Swift.

Yon uses the same rule, with these differences:

| Language | Required field in Yon |
| --- | --- |
| JavaScript, TypeScript, Python, PHP | a field without `=`; a TypeScript `label?:` field is refused (see the amendment) |
| Java, C#, Dart, Kotlin | a field without `=`; Yon has no props, so a plain field without `=` is a dependency too (Java `final`, C# `readonly`, Dart `final`, Kotlin `val` are the usual forms) |
| Go | every named field (Go has no field initializer); embedded fields are skipped |
| Rust | every field |
| Ruby | every `attr_reader`, `attr_accessor` or `attr_writer` field of a `#[NeedsAll]` class; the provider is the PascalCase field name, or `#[Needs]` with `#[Named("...")]`; no `initialize` |

A required Yon field without a provider stops the build with the class, the
field and the type: "Unresolved dependency MissingService: MoneyController
needs field 'money' of type MissingService, but no provider MissingService is
declared".

## Consequences

- A page that forgets a prop fails the build with the service, the field and
  the place to pass it.
- Initialized fields no longer need props, and a prop that a page passes
  anyway no longer sets them, in any language.
- A Go field is plain state unless it is marked `// @Required`, the
  explicit non-null marker that Lombok also honours. The website's Go
  `CatalogService` marks `Features` so that it keeps receiving the catalogue
  from the page (a method argument is limited to 64 KiB, too small for it).
- The website passes `axis` to every snippet and the topic service names the
  axis of every section.

## Amendment: a filled field is never nullable (2026-09-28)

A required field and a `Needs` field are never null when the class runs:
Tachyon fills them first, and it refuses a missing or null value. A nullable
declaration (`label?: string`, `final String? path;`) says the opposite, so
the reader cannot trust it. The build now refuses it with `TY1331`, in both
realms and in every language that has a nullable field form, and it shows the
language's own "filled by the framework" form:

| Language | Refused | Write instead |
| --- | --- | --- |
| TypeScript | `name?: T`, `name: T \| null`, `name: T \| undefined` | `name!: T` |
| Swift | `let x: T?`, `var x: T?`, `var x: T!` | `let x: T` (generated `init?(tacProps:)`); `var x: T!` for a `Needs` field |
| Kotlin | `val x: T?`, a `Needs` field `var x: T? = null` | `val x: T` (generated constructor); `lateinit var x: T` for a `Needs` field |
| Dart | `final T? x;`, `late T? x;` | `late final T x;` |
| C# | `required T? X`, `readonly T? X` | `required T X` in Tac, `readonly T X` in Yon |
| PHP | `?T $x`, `T\|null $x` | `T $x` |
| Python | `x: T \| None`, `x: Optional[T]` | `x: T` |
| Rust | `x: Option<T>` | `x: T` (a Yon provider is `Arc<Provider>`) |

JavaScript, Go, Java and Ruby have no nullable field form, so nothing changes
there. A field that can really be empty is plain state: it has a starting
value (`note: string | null = null`), Tachyon never fills it, and it is not a
required prop. So a TypeScript `label?:` field in a `NeedsAll` class is no
longer optional state: write `label!: string` if the page passes it, or give
it a starting value.

Decisions:

- The check lives in the annotation rule table (`refuses_nullable` on
  `NeedsAll` and `Needs`), next to the constructor refusal of ADR 0056. A
  `NeedsAll` class checks the fields that ADR 0057 calls required in its
  realm (for example a Dart field only when it is `final` or `late`, a Tac C#
  member only when it is `required` or `readonly`, a Yon C# property never).
  A `Needs` field is checked whatever its initial value.
- C# uses `required T X;` (a field or a `{ get; set; }` property), not
  `required T X { get; init; }`: the native companion writes each prop with
  an assignment after the object initializer, and C# refuses to assign an
  `init` accessor there.
- Dart uses `late final T x;`: the generated constructor
  `Service(this.x, ...)` assigns it before any method runs, and the source
  stays valid Dart for editors, which do not see the generated constructor.
- A Rust `Option<T>` field is refused like the other nullable forms, unless
  the struct derives or implements `Default` (then every field is plain
  state).
- `ty migrate fix --write` rewrites a one-line declaration to the non-null
  form; a declaration that spans lines stays for the author.

## Amendment (2026-09-28): an injected field uses the language's variable case

A field that holds an injected class copied the class name in some examples
(`CartStore CartStore;`, `ledgerStore: LedgerStore` in Python). The reader
then sees a type where the language shows a variable. The build now refuses
such a field with `TY1332` and names the spelling to write:

| Language | Case | Example |
| --- | --- | --- |
| JavaScript, TypeScript, Java, Kotlin, Swift, Go | lowerCamel | `cartStore` |
| Dart | lowerCamel, a leading `_` allowed | `cartStore`, `_cartStore` |
| PHP | lowerCamel | `$cartStore` |
| Python | snake_case, a leading `_` allowed | `cart_store` |
| Ruby, Rust | snake_case | `cart_store` |
| C# private or internal field | `_` and lowerCamel | `_cartStore` |
| C# property, public or protected field | PascalCase | `CartStore` |

Decisions:

- Only the case style is checked, not the words: `CartStore ledger` with
  `Named("CartStore")` stays allowed.
- The checked fields are every `Needs` field, every Yon `NeedsAll` field
  without a starting value (Yon has no props, so each one is a dependency),
  and a Tac `NeedsAll` field in JavaScript, TypeScript or Dart whose type (or,
  in JavaScript, name) is a class. A native Tac `NeedsAll` field is a prop.
- C# follows the .NET conventions: private and internal fields are
  `_camelCase`, and accessible members are PascalCase. Go uses unexported
  lowerCamel fields, which the container fills.
- The framework's `behaviors` field (the behavior context that generated
  code calls) keeps its fixed name in every language.
- The JavaScript (lowerCamel) and Ruby (snake_case) name lookups keep
  working, because the fixed spelling is the lookup form of the provider.
- `ty migrate fix --write` renames the field, its uses in the class
  (`this.`, `self.`, `$this->`, `@`, a Go receiver and bare uses in Java,
  Kotlin, C#, Swift, Dart and Ruby) and the Tac view bindings
  (`{cart.CartStore.size}`). It leaves a field alone when the new name is
  already taken or when a bare use of a PascalCase name could be the type.
