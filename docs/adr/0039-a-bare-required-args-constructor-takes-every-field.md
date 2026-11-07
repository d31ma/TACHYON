# ADR 0039: A Bare RequiredArgsConstructor Takes Every Field, and Makes the Service Per Component

- Status: Accepted
- Date: 2026-09-24
- Amends: [ADR 0038](0038-resources-are-autowired-by-spring-names.md) (the
  Tac meaning of `@RequiredArgsConstructor`)
- Preserves: an explicit `@Scope`, a named `@RequiredArgsConstructor` list, Yon
  constructor injection
- Amended by: [ADR 0047](0047-annotation-names-are-plain-english.md): annotations and role folders have plain English names (for example `Needs`, `NeedsAll`, `Store`, `OnStart`, `stores/`); the old names are refused and `ty migrate fix --write` renames them

## Context

A Tac service that took its component's properties had to name them twice:
once as fields, and again in
`@RequiredArgsConstructor({"value":["path","docs"]})`. Lombok's
`@RequiredArgsConstructor` takes no arguments. It builds the constructor from
the fields that need one.

Such a service also had to say `@Scope({"value":"prototype"})`. But a service
that takes a component's properties can only exist once per component. A page
singleton is constructed without any properties.

## Decision

- On a Tac service, a bare `@RequiredArgsConstructor` takes every
  uninitialised field, in declaration order, except the autowired resources.
  The component properties with those names fill them.
- In Rust and C#, fields are always initialised. There, the bare marker lets
  every field take the property with its name.
- A service that declares `@RequiredArgsConstructor` exists once per
  component. The build infers `@Scope` from it. Write `@Scope` only to override
  the inference, or for a per-component service that takes no properties.
- A named list still parses, and it narrows the set of fields.
- Yon does not change. Its constructors inject collaborators. A service that
  autowires them uses `@Autowired` fields.

## Consequences

- The website's services write `@RequiredArgsConstructor` bare and drop
  `@Scope`. `ClipboardService` keeps `@Scope`, because it takes no properties.
- A Tac service that used `@RequiredArgsConstructor` to inject clients now
  exists once per component, and it no longer shares the page's clients. It
  autowires them instead, as the migration guide shows.

## Amendment (2026-09-30): Java and F#

- A Java Tac service with `@NeedsAll` takes every non-private field without
  an initial value from the properties with the same names.
- An F# Tac service takes them as primary constructor parameters (ADR 0073).
