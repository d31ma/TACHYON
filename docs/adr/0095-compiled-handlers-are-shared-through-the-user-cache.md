# ADR0095: Compiled handlers are shared through the user cache

Status: Accepted.

Yon compiles Kotlin, Rust, Dart, Go and Groovy handlers to one file each and
keeps that file in the project's `.tachyon/handlers` cache. Each project used
to compile its own copy. The same handler in a second project, a fresh
checkout or a new test project was compiled again. A Kotlin compile takes
about eight seconds, and these compiles were about half of the core test
suite's time.

## Decision

After Yon builds one of these artifacts, it also writes a copy to
`runtime/handlers-v1` in the cache that `TAC_CACHE_DIR` selects. The copy has
the artifact's name: a digest of the authored source, the model files, the
generated protocol code, the compiler identity, the processor architecture and
the operating system.

When a project's cache has no valid artifact, Yon looks for a shared copy with
the same name while it holds the project's build lock. It restores the copy
only when it is a regular, non-empty file. It never follows a symbolic link in
the shared folder. The restored file goes into the project cache through the
same confined publication as a compiler output. Execution still copies the
artifact from the project cache into a private runtime workspace.

The shared folder keeps the newest 512 entries. A failure to read or write the
shared folder never fails a build: Yon compiles as before. `ty cache clean`
removes the shared folder with the rest of the runtime cache.

C# and F# handlers keep their own multi-file .NET caches and do not use the
shared folder.

Yon also asks a compiler for its version once per process for the same `PATH`
and `JAVA_HOME`, instead of once per handler.

## Consequences

- A warm core test run on the development Mac went from 527 s to 334 s.
- The user cache is now an input to handler builds, as it already is for the
  web builds of C#, Kotlin, Swift and Rust services. Anyone who can write to
  that folder can change the handlers that Yon runs, so it must stay private
  to the developer account, like the rest of the cache.
- A shared copy is trusted by its name. A damaged copy is fixed by
  `ty cache clean`.
