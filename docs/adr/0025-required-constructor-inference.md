# ADR 0025: Required-Field Constructor Inference in Yon

- Status: Accepted
- Date: 2026-09-12
- Amends: ADR 0021 explicit dependency authoring
- Preserves: ADR 0022 behavior ownership, ADR 0024 strict module layout
- Amended by: [ADR 0031](0031-realms-mirror-each-other.md), which mirrors
  inference into Tac and adds `ApplicationEventPublisher` to the reserved
  provider tokens
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them

## Context

Managed Yon types already declare their dependencies as fields. Without
inference, authors repeat those fields in a constructor and in an Inject token
list. That is extra work, and the order or the names can drift apart.
Inference from the source alone can remove this repetition. It needs no
reflection, no runtime package scanning and no persistent container.

## Decision

This decision applies to managed Controller, Service, bare Repository, Client
and Component types. On these types, RequiredArgsConstructor with no arguments
selects the direct fields that are uninitialized and non-static. It selects
them in declaration order. An explicit list of quoted field names can
optionally select or reorder them. When no Inject list is authored, both forms
infer the provider tokens. Both forms lower to the existing explicit
constructor plan.

Matching rules:

- Typed languages require exact concrete provider types.
- Rust requires Arc<Provider> or std::sync::Arc<Provider>.
- JavaScript requires an exact lowerCamel match to a managed provider name.
  This includes acronym conversion, such as HTTPClient to httpClient.

Missing or ambiguous matches fail. Tachyon does not guess. Selected optional,
interface, generic and function dependencies are not supported. Initialized
defaults and static state are kept as they are. They are not injected.

Inference runs over the captured same-language unit that was assembled
explicitly. It never loads or constructs a provider. Existing limits, graph
validation, scopes and runtime adapters still have the final say. The
inferred ABI (application binary interface) is internal. Developers do not
need to author generated constructors or Inject lists.

Explicit Inject and handwritten constructors stay optional alternatives. An
explicit Inject together with an explicit RequiredArgsConstructor field list
keeps its previous positional contract. Do not combine a generated
constructor marker with an authored constructor. When Rust generates a
constructor for zero fields, it uses a braced struct, not a unit struct. This
lets it generate the existing new() construction ABI.

This decision covers only inference for managed Yon roles. It does not add:

- Tac DI (dependency injection);
- configuration binding;
- primitive-value injection;
- interface tokens;
- factories;
- property scanning;
- lazy resolution;
- cross-language dependencies.

Tac and unmanaged behavior types keep explicit construction and ownership.

Formal Helper role and folder validation do not transfer caller-owned values to
Yon DI. A source-local Helper with a proven explicit construction call and no
class Needs, NeedsAll or Lifetime remains caller-owned when its lowered
constructor needs arguments. Go uses a nonempty authored struct literal as
construction proof. Field Needs may describe its existing explicit constructor;
it does not select a managed provider. Capture precedes behavior and constructor
lowering so that schema and annotation consumption cannot erase ownership.

Zero-argument Helpers retain their existing managed identity and lifecycle.
Caller-owned Helpers cannot carry managed OnStart, OnStop or OnEvent hooks;
requesting them through DI or a gateway reports their path and the required
class-level opt-in. Service and Converter constructor checks, duplicate names
and bounded provider counts remain unchanged. Comments, strings, factory method
references and similar names are not construction proof.


## Consequences

- The recommended provider/controller examples need only role markers,
  RequiredArgsConstructor, direct fields and ordinary methods.
- Field declaration order matters, unless the author overrides it
  explicitly.
- JavaScript naming is strict and checked for collisions. Typed languages
  avoid guessing by name, because they resolve concrete types.
- Source diagnostics reject unsupported inference before invocation. Existing
  explicit wiring is a deliberate alternative. It is not a silent fallback.
- Role filenames, plural folders and explicit module manifests are still
  required. Dependency inference does not imply module discovery.

## Acceptance

Parser tests cover:

- all eight Yon spellings;
- empty and explicit field selections;
- ordering;
- exclusion of defaults and static fields;
- concrete-type and acronym matching;
- malformed or unsupported fields;
- preservation of explicit compatibility forms.

Integration tests use supervised runtime paths. They exercise:

- generated constructors;
- diamond identity (two dependents that share a provider get the same
  instance);
- transient scope;
- declaration/list order;
- existing response behavior.

Website contracts require eight complete role-layout examples without
authored Inject lists or constructors. Toolchain failures are still reported
separately. Authored examples are not evidence of runtime success.
