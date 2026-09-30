# Outbound classes live in remotes

New apps use `client/remotes/` and `server/remotes/` for outbound HTTP clients,
socket clients and gateway adapters. The outer client and server folders still
identify Tac and Yon. `HTTPClient`, request, socket and gateway annotations and
class names keep their existing meaning.

Both `clients/` and `remotes/` compile in each realm. There is no forced move.
Distinct classes may live in both during migration. A class with the same name
in both is a duplicate identity and is refused rather than selected by folder.

Preview the existing migration plan before writing:

```sh
ty migrate fix path/to/project
ty migrate fix path/to/project --write
```

The plan moves each old role file to its realm's `remotes/` folder, retains its
class name and current annotations, and rewrites paths for successfully moved
files. A taken destination produces a manual step: both files stay intact and
references to the unmoved file retain the old path. Existing migrations may
also propose unrelated older annotation or layout repairs; review the full plan.

Recognized paths include full realm paths and relative `../clients/` and
`./clients/` paths naming moved files with their extensions. Review custom
loaders, extensionless paths, external tooling and dynamic path construction
manually. No external app or original worktree is modified by this framework
change. New scaffolds and published examples use `remotes/`.
