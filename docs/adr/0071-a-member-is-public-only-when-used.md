# ADR 0071: A Member Is Public Only When It Is Used

- Status: Accepted
- Date: 2026-09-28
- Extends: [ADR 0069](0069-design-rules-are-build-errors.md) (design rules
  are build errors) with a fifth rule
- Preserves: [ADR 0063](0063-services-and-controllers-are-cohesive.md) (a
  public method only coordinates) and
  [ADR 0070](0070-coverage-means-a-test-called-the-method.md) (coverage means
  a test called the method)

## Context

A public member is a promise: another part of the application may call it.
When nothing calls it, the promise is false. A reader must search the whole
project to learn that the member is only a helper, and the cohesion rule
(ADR 0063) treats it as a coordinator, so it may hold no decision. Many
examples and services kept helpers, counters and constants public only
because public is the default in most languages. Some members were public
only because a test called them, so the test checked a helper instead of the
behaviour that the view or another class sees.

## Decision

The build refuses a public member of a class when nothing outside the class
uses it (`TY1333`). The check is the fifth design rule of ADR 0069 and runs
in the same module and at the same time.

### Which classes

The classes in `services`, `helpers`, `controllers` and `utils` of each
realm. Models, entities, errors and test files are not checked.

### What counts as a use

A member stays public when one of these uses it:

1. **The view.** A Tac view (`client/pages/**/tac.html` and
   `client/components/**/tac.html`) reads or calls `instance.member`, where
   `instance` is the service's instance name (`CartService` is `cart`). The
   name is compared without case and `_`, so `cart.itemCount` matches
   `ItemCount` in C# or Go and `item_count` in Ruby, Python and Rust.
2. **Another class of the same realm.** A source file that names the class
   (its name, its instance name or a field name for it) accesses the member,
   such as `this.priceService.total()` or `PriceUtils.label(total)`. Other
   language files of the same class (companions) do not count, except for
   a JavaScript or TypeScript sibling of a C# service in `client/services/`.
   A direct `this.member` access in that sibling keeps the C# member public
   because the page bridge needs public members. The files must have the
   same path without their extensions. This exception does not apply to
   server services, other role folders, same-language self-use or tests.
3. **Tachyon.** The member has an annotation (`@OnEvent`, `@GetRoute`,
   `@OnStart`, `@Cached`, and every other marker, also an old one that
   `ty migrate fix` renames); one of its parameters has an annotation, such as
   `@Check`; it is a field that Tachyon fills (a field without a starting
   value, or any field that `Needs` or `NeedsAll` fills); a setup or
   configuration class hands the class to Tachyon (a cache or data adapter);
   a `Gateway` names the class file, such as
   `{"class":"server/services/ReportService.java"}`, so a gateway in any
   language can call its methods; it is `identify` of the `@IdentityProvider` class; or it is a name that
   the runtime or the language calls: `hotState`, `hotDispose`,
   `restoreHotState`, `toString`, `toJSON`, `equals`, `hashCode`, Go
   `String` and `Error`, Ruby `to_s`. Constructors, `override` methods,
   methods of `impl Trait for Type`, Python and PHP `__` methods and the
   methods of a class that names a base type are not checked either.

A call from a test does not count. A test checks the member through the
public member that uses it.

### The private form of each language

| Language | Private form |
|---|---|
| TypeScript, Java, Kotlin, C#, Swift, PHP | `private` |
| JavaScript | a name that starts with `_` (`#name` also counts) |
| Dart, Python | a name that starts with `_` |
| Ruby | `private` (`private def total`, `private attr_reader :total`, or below a `private` line) |
| Go | a name that starts with a lowercase letter |
| Rust | no `pub` |

JavaScript uses `_name`, not `#name`. Tachyon reads JavaScript classes in
several places (the `NeedsAll` field reader, the Tac runtime that walks
instance fields, the behaviour wrappers). A `#name` method already works, and
the `NeedsAll` field reader now skips `#name` fields, but `_name` needs no
support from any reader, so the fix and the help use it.

### Migration

`ty migrate check` lists every break. `ty migrate fix --write` makes the
simple cases private in the realms where the rule is on: it writes `private`
or removes `pub` in place, and for a rename form (`_name`, a lowercase Go
name) it renames the member and its uses through `this`, `self`, the class
name or the Go receiver. It leaves a member alone when the new name is
taken, when a use is unclear (a bare name in Dart or Go that could be a
local), or when a Ruby line declares several readers.

The switch is `tac.visibility.check` in `client/manifest.json` and
`visibility.check` in `server/application.json`. The rule is on by default,
like the other four design rules; `false` turns it off for one realm while
it migrates. The website, its examples, the corpus, the `ty init` templates
and the test fixtures pass the rule.

## Consequences

- A reader sees at once which members other code relies on.
- Examples show only the members that their views or controllers use. Some
  examples gained a small view or controller, because a service that nobody
  calls shows nothing.
- Dead members become visible: a private member that nothing calls fails the
  coverage check of ADR 0070, so it is deleted instead.
- Tests call public members only. A test that reached into a helper now
  checks the result that the view or the caller sees.
- The check reads tokens, not a parse tree, and it counts a use when another
  file names the class and accesses a member of that name. A common member
  name in a file that names the class can keep a member public that is not
  really used; the rule never refuses a member that is used.
