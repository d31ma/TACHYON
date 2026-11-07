# ADR 0095: Native Go Packages Live in server/packages

- Status: Accepted
- Date: 2026-10-03
- Works with: [ADR 0091](0091-role-folders-allow-nesting-and-require-explicit-registration.md)
  (role folders need explicit markers; this ADR gives plain native Go code a
  place outside them), [ADR 0064](0064-code-is-not-duplicated.md) (plain
  utilities) and [ADR 0043](0043-models-hold-data-types.md) (models are data
  types of package main)

## Context

A Go Yon back end for front end (BFF) usually has its own Go packages: a git
client, a JSON schema checker, secrets, a repository layer, spec types and
commands. Each package has its own package name and imports the others with
a module path. A consumer app kept these packages in `server/helpers/git`,
`server/services/*`, `server/repositories`, `server/utils`,
`server/models/spec` and `server/commands`. Almost every bundle error of
that app came from them: `TY1363` because a concrete type in a role folder has
no marker, and `TY1310`, `TY1315`, `TY1318` and the design rules (`TY1327` to
`TY1333`) because the class-file conventions do not fit Go packages.

Go Yon code had no place for such packages. Every Go handler is built as one
`package main` in a module called `yonhandler`, with `GOPROXY=off`.
`server/shared/` joins package main and refuses another package clause.
Models must be package main as well. So a controller could not import the
app's own packages at all.

Spring has the same split. A library on the classpath is not a bean: the
container manages only the classes that a scan and an annotation register.
ADR 0091 keeps that rule for role folders: a folder name never registers
code, and there is no switch that registers unmarked code.

## Decision

`server/packages/` holds one native Go module. `server/packages/go.mod` names
the module with a `module` line. Each folder below it is an ordinary Go
package. Controllers, services and other Go role files import those packages
with the module path, and the packages import each other the same way.

Tachyon stages the whole folder next to every Go handler build, as
`packages/` beside the handler. It adds `require <module> v0.0.0` and
`replace <module> => ./packages` to the generated handler module, so the
build stays offline. The staged files are part of the handler digest, so a
change to a native package rebuilds the handler.

Code in `server/packages/` is not a role. It needs no marker, and Tachyon
does not apply the role folder, class-file, file-name, cohesion, duplicate or
design rules to it. The secret and code safety checks still run, because they
protect the app whatever the folder.

These limits fail with `TY1310`:

- A Go file in `server/packages/` without `server/packages/go.mod`.
- A `go.mod` without a `module` line, or with any directive other than
  `module`, `go` and `toolchain`. Yon builds Go offline, so a native module
  cannot require other modules yet.
- A second `go.mod` inside the folder.
- Go code there that carries a Tachyon annotation. An annotated type belongs
  in its role folder.
- A source file of another Tachyon language in the folder.

Other files, such as JSON schemas read with `go:embed`, are staged when they
are UTF-8 text.

The `TY1363` help for a Go file and the `TY1310` help for a Go model with
another package name point to `server/packages/`.

## Consequences

- An app moves its native packages from role folders into
  `server/packages/` and changes their import paths to the module path of
  `server/packages/go.mod`. Role folders keep only marked roles.
- Native packages are not injected, do not take part in role discovery and
  cannot be named by `Needs`. A role that needs one calls it directly.
- A native type used as a route parameter or result is not a Tachyon model,
  so typed mapping and `TY2015` model checks do not read it.
- Third-party Go modules stay unsupported. Supporting them needs a module
  cache or vendor policy and is a separate decision. The amendment below
  makes it: vendored modules only.
- Other Yon languages keep their own module rules; this folder is Go only.

## Amendment (2026-10-03): vendored third-party modules

A real back end for front end needs third-party modules, such as cloud SDKs
and `gopkg.in/yaml.v3`. The native module may now require them, but only
vendored: Tachyon still never downloads a module.

- `server/packages/go.mod` may hold `require` lines, in single or block form,
  next to `module`, `go` and `toolchain`. Other directives, such as
  `replace`, still fail with `TY1310`.
- The app runs `go mod vendor` in `server/packages/` and commits `go.sum` and
  `vendor/` with `vendor/modules.txt`. When `go.mod` requires a module and
  either file is missing, discovery fails with `TY1368`, whose help tells the
  author to run `go mod vendor`.
- When a native module exists, Tachyon stages the whole folder as the main
  module of each Go handler build and writes the handler program into its
  reserved `yonhandler/` folder, importing the invocation package as
  `<module>/yonhandler/yoninvocation`. This replaces the generated
  `require` and `replace` wrapper: `vendor/modules.txt` must match the main
  module's `go.mod`, so the user's own `go.mod` has to be the main module.
  Handler code therefore compiles with the module's `go` version. A package
  in `server/packages/yonhandler/` fails with `TY1310`. Without a native
  module, the handler keeps its generated `yonhandler` module.
- Every Go handler build sets `GOFLAGS=-mod=vendor` next to `GOPROXY=off`,
  so Go reads modules only from `vendor/` and never from a module cache or the
  network. Go checks `vendor/modules.txt` against `go.mod`; an inconsistent
  vendor folder or an import that `vendor/` does not provide fails with
  `TY1368` and Go's own detail, with the temporary build folder shown as
  `server/packages`.
- Files are staged as bytes, so vendored binary files (for example embedded
  protobuf defaults) work, and the native files no longer need to be UTF-8.
  A Sha256 fingerprint of every staged native file, including `go.sum`,
  `vendor/modules.txt` and each vendored file, is part of the handler digest.
- `vendor/` is third-party code. The `server/packages/` file checks, and the
  role, file name, cohesion, duplicate and design rules, skip it. The secret
  check and the code safety checks (eval, query, service reach and outgoing
  hosts) skip it too. The author cannot fix a finding in code that the next
  `go mod vendor` rewrites, and SDKs legitimately carry documented example
  keys, their own service addresses and process calls. Review happens when
  `vendor/` changes: in code review, and with `go mod verify` and
  `govulncheck` where a module cache exists. Authored native packages keep
  every safety check.
- `vendor/` counts toward the `server/` snapshot limits (4,096 files and
  64 MiB). The AWS, Azure and Google Cloud SDKs that one consumer app uses
  vendor to about 2,500 files and 37 MB, with no file above 1 MiB.

Tachyon does not prove that `vendor/` matches the upstream modules: `go.sum`
hashes cover whole module archives, and Go does not compare vendored files
with them. `docs/THREAT_MODEL.md` records this limit.
