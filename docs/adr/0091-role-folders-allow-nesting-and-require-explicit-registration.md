# ADR 0091: Role Folders Allow Nesting and Require Explicit Registration

- Status: Accepted
- Date: 2026-10-01
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (shared realm
  folders) and [ADR 0044](0044-an-annotated-file-holds-one-class.md)
  (behavior-only Tac services)
- Preserves: [ADR 0043](0043-models-hold-data-types.md) (data-only models),
  [ADR 0047](0047-annotation-names-are-plain-english.md) (marker names),
  [ADR 0048](0048-yon-routes-come-from-annotations.md) (annotation routes),
  [ADR 0064](0064-code-is-not-duplicated.md) (annotation-free utilities) and
  [ADR 0088](0088-remotes-are-the-canonical-outbound-role-folder.md)
  (`clients/` stays a compatibility name for `remotes/`)
- Related: [ADR 0092](0092-settings-and-validation-data-live-in-models.md)
  (settings and validation data move from `helpers/` to `models/`)
- Amended by: [ADR 0112](0112-a-delegate-takes-on-the-methods-of-its-service.md): the Helper role is now `Delegate`, and its role folder is `delegates/`; `helpers/` stays as a folder that is not a role, for `Gateway` programs and behavior-only holders

## Decision

The client and server role folders stay the place where Tachyon finds roles.
Inside a role folder, authors may organize files in nested folders, such as
`client/services/billing/InvoiceService.ts`. Nested services and controllers
find their collaborators across the whole realm. Tachyon uses the real
relative paths when it composes source, rebases imports and hashes services.
A nested folder does not create a new injection namespace, so two variants of
the same service in one language are still an error. Discovery keeps the
existing rules: it opens no symbolic links, reads files in a fixed order, and
stops at 4096 entries and 32 folder levels.

A concrete class, struct, object or record in `services/`, `stores/`,
`remotes/` (or `clients/`), `setups/`, `entities/`, `helpers/` or
`converters/` needs that folder's marker. A server controller needs
`RequestRoute`, `SocketRoute`, `ErrorHandler` or `IdentityProvider`. The
marker spelling and the languages that support it stay language-specific.
A missing marker fails with `TY1363`. The message names the file and the
marker spelling for its language. The placement, class suffix, one-class and
role-specific checks still apply.

This replaces the ADR 0044 rule that let a Tac service with only behavior
annotations leave out `Service`. There is no switch that registers concrete
unmarked code. An existing app adds the marker to a class that really has the
role, or moves the class to its plain folder.

Abstract classes, interfaces, protocols, traits, type aliases, files that
only re-export, and assets are not roles. Compiler output is not authored
source. Tests outside application discovery stay fixtures. Models, plain
shared code, native errors and annotation-free static utilities keep their
own rules. Helpers are formal roles: a `*Service` class in `helpers/` fails
the suffix check and belongs in `services/` with `Service`. Where a language
has no Helper support yet, a Helper fails with a clear message instead of
being accepted unchecked.

`ty migrate check` reports each missing marker as a `role.marker` finding.
It does not add markers, build Store arguments, choose a controller route or
give helper code an injectable lifetime. The author decides the role and moves
misplaced collaborators. `clients/` stays accepted; folders with retired names
keep their migration errors.

Settings and validation data holders that used to sit in `helpers/` move to
`models/`. ADR 0092 describes that move.

## Native Rust services and their dependencies

A native Rust service is built with the other Rust services of the same
language that it needs, in the namespace of the service that owns them. Their
constructor metadata is kept while their role markers are removed, and the
authored files are checked before that step. The browser keeps services in
separate modules as before. A shared native provider graph is needed for
singletons shared across pages and for dependency lifecycle hooks; building a
dependency inside its owner does not give those guarantees. Choosing a trait
implementation needs an explicit typed binding; Tachyon does not infer one.

## Caller-owned C# Tac Helpers

A C# Tac Helper works when its caller creates it. A marked Helper keeps its
typed constructor and must be created with `new` outside its own class. The
Setup or Service that creates it owns any context it receives. Helper
provider and lifetime declarations on the class, injected Helper dependencies
and Helper lifecycle or event hooks are refused before the authored metadata
is removed. Field `Needs` constructor lowering works only for typed
constructors that are called explicitly. C# has no managed Helper injection
and no automatic Helper cleanup.

## Go Tac Helpers

A Go Tac Helper is built into the page package with the services that use
it. A Go service gets a Helper in one of two ways. A `// @Needs` field such as
`step *StepHelper` gives the service its own new Helper before OnStart runs,
and the service can also create one itself, for example `&StepHelper{}`. A Go
Helper is never shared between services and is not exposed to the view unless
its field is exported. A Go Helper has no Needs, NeedsAll or Lifetime of its
own (TY1308) and no lifecycle or event hooks (TY1405); it receives what it uses
through method parameters or fields that its service sets.

## Swift Tac Helpers — 2026-10-17

A Swift Tac Helper is built by the field that needs it. Tachyon writes
`@Needs private let cartHelper: CartHelper` as
`private let cartHelper: CartHelper = CartHelper()`, in a service and in a
Helper under `client/helpers/` at any depth, so a Helper may need other
Helpers. Each class that needs a Helper owns its own instance, which lives
and goes away with that class. A Swift Helper therefore has an `init()`
without arguments. It has no lifetime of its own and no `OnStart`, `OnStop`
or `OnEvent`; the event check refuses those with `TY1405`, as it does for any
type that is not the service. Named does not select a Swift Helper.

Evidence: `tac_web::swift_helper_tests` builds a Swift prototype service that
needs a Helper which needs another Helper, runs it under Node, and checks that
each service instance and its replacement have their own Helpers.

## Amendment (2026-10-03): one structure check before the build

The rules of this ADR, ADR 0067 (file names) and ADR 0064 (utilities) run as
one structure check. Project discovery reads every file of both realms at any
depth and checks each file before the compiler writes output or starts a
toolchain (`dart`, `cargo`, `go`, `dotnet` and the others). It collects the
problems of every file into one report, so one run lists all of them. Each
diagnostic names the file, points at the declaration or import, and gives the
fix in the language's own spelling. Within one file the checks stop at the
first problem, because the later checks read the class that the earlier ones
accept.

The check covers, per role folder and language:

1. the role marker of the folder (`TY1363`);
2. the role suffix of a marked class and its role folder (`TY1310`);
3. the file name, which is the class name in the language's file name style
   (`TY1318`, `TY1310`);
4. one primary class per role file, or the language's form of it: a Go struct
   with its methods, a Lua class table, an F# type, a Rust type with its
   `impl` blocks (`TY1310`);
5. dependencies that point down and never reach a controller (`TY1327`). A
   utility class in `client/shared/utils` or `server/utils` names no role class:
   every class of its realm may call it, so it cannot depend on one. The
   other design rules check `client/shared/utils` too (ADR 0069 amendment);
6. realm boundaries: a JavaScript, TypeScript or Dart file never imports a file
   of the other realm with a relative path (`TY1369`). Client code reaches the
   server through an HTTPClient; server code never imports client code.

Models, utilities, tests, generated files, the native Go module in
`server/packages` (ADR 0095) and the support files in `client/shared` keep
their own rules. A folder that is not a role folder never needs a marker.
`ty migrate check` lists missing markers (`role.marker`), imports of the other
realm (`role.realm_import`) and the dependency findings; `ty migrate fix
--write` renames a file whose class has another name.

## Amendment (2026-10-17): Kotlin Tac Helpers

A Kotlin Tac service or Helper may name a Helper from `client/helpers` in a
`@Needs` field. The field becomes `private val rates: TaxRateHelper =
TaxRateHelper()`, as a Kotlin client field does, so each class that needs a
Helper makes its own new one, in the browser, in a native build and in a
test unit. A Helper therefore has a constructor without parameters. It has
no shared lifetime and no lifecycle of its own, so a Kotlin Helper that
carries `@NeedsAll`, `@Lifetime`, `@OnStart`, `@OnStop` or `@OnEvent` stops
the build at the Helper file and says to mark each needed field with
`@Needs` and move lifecycle and event methods to the Service. A Kotlin test
unit replaces a needed Helper with an `@Fake` field, as it does a client.
Swift still refuses Helper.

## Amendment (2026-10-18): Java Tac Helpers

A Java Tac service or Helper may name a Helper from `client/helpers` in a
`@Needs` field, as Kotlin does. The build writes
`@Needs private final TaxHelper helper;` as
`private final TaxHelper helper = new TaxHelper();`, so each class that
needs a Helper makes its own new one. A Java Helper may need other Helpers.
It takes no `@NeedsAll`, `@Lifetime`, `@OnStart`, `@OnStop` or `@OnEvent`,
and the build stops such a Helper at the Helper file (ADR 0083 amendment of
2026-10-18). With Java, every Tac language accepts Helper: the rule table's
`TAC_HELPER` is now the full `TAC` list.

## Amendment (2026-10-18): C# and F# Tac Helpers are built by Needs

A C# or F# Tac service or Helper may name a Helper from `client/helpers` in
a Needs, as Java and Kotlin do. This replaces "C# has no managed Helper
injection" in "Caller-owned C# Tac Helpers" above.

- C#: the build writes `[Needs] private readonly TaxHelper _tax;` as
  `private readonly TaxHelper _tax = new TaxHelper();`. The field uses the C#
  field case (`TY1332`).
- F#: a primary constructor parameter `type QuoteService(tax: TaxHelper) =`
  becomes `type QuoteService() =` with `let tax = new TaxHelper()`. A
  property `[<Needs>] member val Tax : TaxHelper = Unchecked.defaultof<TaxHelper> with get, set`
  becomes `member val private Tax : TaxHelper = new TaxHelper() with get, set`,
  so the view does not see it.
- So each class that needs a Helper gets its own new one. A Helper is not
  shared. A Helper may need other Helpers in the same way, so it has a
  constructor without parameters when a Needs builds it.
- The same writing happens for the services that a Tac test unit or a
  native build composes, so a test unit builds a service with
  `new CheckoutService()` and the service has its Helpers.
- A C# or F# Helper still takes no NeedsAll, Lifetime, OnStart, OnStop or
  OnEvent, and an F# Helper's `[<Needs>]` property names another Helper:
  the build stops with `TY1308` at the Helper file.
- A caller may still build a Helper with `new`, as before.
- Still standing: every C# or F# Helper in `client/helpers` needs an owner
  in each service build that composes it, through a Needs or a `new` call
  (`TY1308` otherwise), so a Helper that one service needs and another does
  not stops the other service's build. The page program (one .NET program
  per page) is the step that removes this.

Evidence: `tac_web::dotnet_helper_tests` builds a C# and an F# browser
service that need a Helper which needs another Helper, runs them under Node,
and checks that each service has its own Helpers and that the Helpers are
not view properties. `client_tests::dotnet_helper_unit_tests` runs a C# and
an F# test unit with two services that each own their Helper, and
`annotation_modules::helper_callers_tests` checks the written fields and the
remaining refusals.

## Amendment (2026-10-18): a C# or F# Helper only joins the builds that name it

This removes the "Still standing" bullet of the C# and F# amendment above.
A C# or F# service build now keeps only the Helpers from `client/helpers`
that its own classes name: the service, the Setups, clients and Stores it
composes, and the Helpers those name in turn. A Helper that no class of the
build names is left out of that build, so a Helper that one service needs
no longer stops the build of a service on the same page that does not need
it. A Helper that the build does name still needs an owner, through a Needs
or a `new` call, and stops the build with `TY1308` without one.

Evidence: `tac_web::dotnet_helper_tests` builds a C# and an F# page where
one service needs a Helper and the other does not, and runs both under
Node; `annotation_modules::helper_callers_tests` and
`annotation_modules::csharp_test_helpers_tests` check that an unnamed Helper
stays out of a build and that a named Helper without an owner is refused.
