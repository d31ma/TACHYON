# ADR 0041: Models Hold Types

- Status: Accepted
- Date: 2026-09-24
- Amends: [ADR 0031](0031-realms-mirror-each-other.md) (which folders a realm
  has)
- Preserves: every role folder, its naming rules and its discovery, and the
  fact that a realm ships only what its services and pages use
- Amended by: [ADR 0043](0043-models-hold-data-types.md) (models in every
  service language) and
  [ADR 0044](0044-an-annotated-file-holds-one-class.md), which moves every
  type without a role out of annotated files and into `models/`

## Context

The role folders hold types that the framework registers: services,
repositories, clients, configurations, entities, delegates and mappers. Each
file there declares one role-annotated type.

TypeScript services also use types that have no role: interfaces and type
aliases that describe data or a collaborator, for example the shape of a
guide or the `publishEvent` method of the event publisher. These types had
no place of their own. Each service declared them at the top of its own file,
so two services could not share one, and a reader had to skip them to find
the service.

The compiler ignored any folder that it did not know under `client/`, so a
project could not put these types in a folder of their own and have the
compiler check that folder.

## Decision

- `client/models/` and `server/models/` hold type-only TypeScript files. A
  models folder is not a role folder.
- A models file is a `.ts` file. Each top-level statement is an `interface`,
  a `type` alias or an `import type`, with or without `export`.
- Other files use a model with a type-only import:
  `import type { Guide } from '/models/Guide.ts'`. `/models/` is the realm's
  models folder, as `/shared/` is its shared folder. TypeScript removes the
  import, so no runtime loads the models file.
- Project discovery reads `client/models/` as it reads the role folders, and
  checks every models file with diagnostic TY1310:
  - A role annotation fails with the existing role layout message, which
    names the role folder that the type belongs in.
  - A class, a function, a constant, an enum, a value import or any other
    statement fails. The message says to move the code to a role folder or to
    `shared/`.
  - A file in another language fails.
- The compiler never composes, bundles or runs a models file. When it
  composes role files into one unit, it merges their `import type` statements
  into type-only imports, so the imports stay removable.
- The website's service interfaces move into `website/client/models/`.

## Consequences

- Services and other files share one declaration of a type instead of
  repeating it.
- A project that already had a `client/models/` or `server/models/` folder
  with other code in it stops building until that code moves. The diagnostic
  names the folder to move it to.
- A models file is TypeScript only. JavaScript services describe types with
  JSDoc, and the other languages declare types in their own role files.
- The check reads statements line by line. It accepts the usual layouts of
  interfaces and type aliases, including aliases that continue on the next
  line, and it rejects any other statement.
