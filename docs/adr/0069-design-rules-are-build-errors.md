# ADR 0069: Design Rules Are Build Errors

- Status: Accepted
- Date: 2026-09-27
- Preserves: [ADR 0047](0047-annotation-names-are-plain-english.md) (names
  are plain English), [ADR 0063](0063-services-and-controllers-are-cohesive.md)
  (methods are cohesive) and [ADR 0064](0064-code-is-not-duplicated.md) (code
  is not duplicated)

## Context

A reader who is new to a project learns it from its classes. Four mistakes
make that hard, and a reviewer often misses them:

1. Classes that need each other in a loop, or a store that calls up into a
   service. Nobody can say which class is built first or which one owns a
   rule.
2. Clipped names such as `usr`, `cfg` or `calcTot`. A reader who learned
   English as a second language cannot look them up.
3. Long parameter lists and classes that need many other classes. They hide
   a missing model or a class that does two jobs.
4. Magic values: the same text or number written in several places, or a
   bare number in a condition. A change misses one copy, and the number does
   not say what it means.

The shared engineering instructions already ask this of Tachyon's own code.
Spring Boot refuses circular references by default, and ArchUnit checks
layers. Tachyon checks the same things when it builds, in every Tac and Yon
language.

## Decision

The build checks the classes in the role folders of `client/` and `server/`
(`services`, `controllers`, `stores`, `repositories`, `clients`,
`converters`, `mappers`, `configurations`, `setups`, `workers`, `features`,
`entities`, `models`, `errors`, `utils`, `roles`), and the `@Helper`
classes in `helpers` (the role was `@Component` in `delegates`, ADR 0047). Test
files, views, `shared/` and the other files in `helpers/` are not checked. `ty build`, `ty preview`, `ty
serve`, `ty bundle` and `ty test` refuse a break; `ty migrate check` lists
every break with its line.

| Rule | Code | What the build refuses |
|---|---|---|
| Dependencies point down | `TY1327` | A loop of classes that need each other; a class that needs a class of a higher layer; any class that needs a controller |
| Plain-English names | `TY1328` | A class, method, field or parameter name with a word that is not English |
| Small signatures | `TY1329` | A method with more than 4 parameters; a class that needs more than 5 other classes |
| Named values | `TY1330` | A text or number (not 0, 1 or -1) written twice or more in a class; a number (not 0, 1 or -1) in a condition |

### 1. Dependencies point down

A class needs another class when its source names it: the type of a field, a
`Needs` list, or the field name that `NeedsAll` reads (`orderStore` in
JavaScript, `order_store` in Ruby and Python). The layers are:

- controllers are entry points; nothing may need a controller;
- services may need services, helpers, stores, repositories and clients;
- helpers (`@Helper` classes in `helpers/`) may need helpers, stores,
  repositories and clients, but not a service;
- stores, repositories and clients are data access; they may need other
  data access, but not a service or a helper;
- utilities may be used anywhere (they are not injected);
- Tac views are entry points too: they use services by instance name.

A loop is refused even when every step is allowed. The message names the
whole loop, such as `OrderService -> PaymentService -> OrderService`, and the
help says how to break it: move the shared code into a new service, or send
an event instead of calling back. Each realm has its own graph.

### 2. Plain-English names

Each name is split into words at case changes, `_`, `-` and digits
(`calcTotal` is `calc` and `total`). Every word must be one of:

- a word of the English word list that `ty` ships (SCOWL, `en_US`, with its
  endings; British spellings such as `colour` and `catalogue` also count);
- a short technical word: `id`, `url`, `uri`, `html`, `css`, `json`, `api`,
  `http`, `https`, `ui`, `db`, `sql`, `jwt`, `uuid`, `io`, `os`, `xml`,
  `csv`, `pdf`, `svg`, `config`, `utils`, `async`, `sku` and a few more;
- a word of the project list: `tac.names.words` in `client/manifest.json`
  and `names.words` in `server/application.json`, for brand and product
  names;
- two English words written together, such as `username` or `webhook`.

A clipped word is refused even when the word list has it, and the help gives
the whole word: `usr -> user`, `cfg -> config`, `calc -> calculate`,
`msg -> message`, `idx -> index`, `err -> error`, `tmp -> temporary`,
`val -> value`, `str -> text`, `docs -> documents` and more. Names that the
language or a base type chooses are not checked: constructors, Python and PHP
`__` methods, `override` methods, methods of `impl Trait for Type`, and the
methods of a class that names a base type.

### 3. Small signatures

A method takes at most 4 parameters. More means some of them belong
together: put them in a model. Constructors are not counted (the next limit
covers them), nor is `self`, `this` or `cls`. A parameter that Tachyon fills
from the request (`FromPath`, `FromQuery`, `FromBody`, `FromHeader`,
`FromCookie`) does not count either, because each one names a part of the
request and a route cannot group query values into one model.

A class needs at most 5 other classes: the classes of rule 1 plus the
`DataSource`, `Redis` and `ApplicationEventPublisher` resources. More means
the class does too much: split it.

### 4. Named values

A literal text or number is named when it is the whole value of a named
declaration (a class constant, a field, a local variable or a parameter
default, also inside a list or a map), or when a method only returns it
(the Go form: `func (CartService) limit() int { return 20 }`). A literal in
the arguments of a call is named only by a constant-style declaration
(`const`, `static`, `readonly`, or an `UPPER_CASE` name). Every other literal
is loose. The build refuses:

- a loose text or number (not 0, 1 or -1) that appears twice or more in one
  class; and
- a loose number (not 0, 1 or -1) in a condition: next to `<`, `>`, `==`,
  `!=`, `<=`, `>=`, or inside the condition of `if`, `elif`, `elsif`,
  `unless`, `while`, `until` or `guard`.

The empty text, annotation arguments, `Annotated[...]` in Python, imports,
models and entities (their field defaults are data) and test code are not
checked. Text that is used once, such as a log message, is fine. The help
shows a class constant in the language of the file, such as `static
CART_CHANGED = 'cart:changed';` in JavaScript or `private const string
CartChanged = "cart:changed";` in C#.

### Migration switches

Each rule has one switch per realm, in the same place as the cohesion switch:
`tac.<rule>.check` in `client/manifest.json` and `<rule>.check` in
`server/application.json`, where `<rule>` is `dependencies`, `names`,
`signatures` or `constants`. `false` turns the rule off for that realm while
a project migrates. `ty migrate check` ignores the switches, so it keeps
listing what still needs a change.

### Implementation

The check is one module, `annotation_layout/single_class/design_check`. It
reads each file with the one-class layout token reader and the method
readers of the cohesion check (ADR 0063), and it reads the project files like
the duplication check (ADR 0064). The word list is
`design_check/english_words.txt`, made from the SCOWL `en_US` Hunspell
dictionary (2020.12.07) with its affix rules; its notice is in `NOTICE`.

## Consequences

- Loops and upward calls are found when the build runs, not in review.
- Names can be read without guessing. A domain word goes into the project
  list once, instead of being allowed everywhere.
- Examples grow a little: a repeated key becomes a constant or moves into one
  private method, and a repeated argument becomes a local variable.
- The checks read tokens, not a parse tree. A class is found by its name in
  the source, so two classes with the same name in one realm count as one.
- A Tac service field must not be named after the instance name of another
  service (a `documentation` field next to `DocumentationService`): Tac reads
  such a field as an injected service, not as a property.
- The same four rules will apply to Tachyon's own code through
  `tools/cohesion-scan`; that is a separate change.
