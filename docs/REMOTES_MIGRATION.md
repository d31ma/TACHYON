# Outbound classes live in remotes

New apps put outbound classes in `client/remotes/` and `server/remotes/`.
Outbound classes are HTTP clients, socket clients and gateway adapters. The
outer `client/` and `server/` folders still mean Tac and Yon. The `HTTPClient`,
request, socket and gateway annotations keep their meaning, and so do class
names.

Both `clients/` and `remotes/` compile in each realm, so you do not have to
move anything now. During a migration, different classes may live in both
folders. Two classes with the same name, one in each folder, are a duplicate.
The build refuses them; it does not pick one by folder.

First look at the migration plan, then write it:

```sh
ty migrate fix path/to/project
ty migrate fix path/to/project --write
```

The plan moves each old file to the `remotes/` folder of its realm. It keeps the
class name and the annotations, and it updates paths that point to moved files.
If a file with the same name is already in `remotes/`, the plan lists a manual
step. Both files stay as they are, and references to the file that did not move
keep the old path. The plan may also list older, unrelated annotation or layout
fixes, so read the whole plan before you write it.

The migration updates full realm paths and relative paths such as
`../clients/Name.js` and `./clients/Name.js` when they name a moved file with
its extension. Check these yourself: custom loaders, paths without an
extension, external tools, and paths that code builds at run time. New
projects from `ty init` and the published examples use `remotes/`.
