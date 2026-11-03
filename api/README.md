# Tachyon Machine Contracts

This directory is the official source of Tachyon's versioned machine
contracts. Code, generated artifacts, documentation and examples must match
these schemas.

Each contract has:

- a directory for its major version, which never changes;
- a Draft 2020-12 JSON Schema with a stable `$id`;
- one example that the schema accepts;
- one example that the schema rejects, which proves that validation works.

Inside one major version, a change must only add things and must stay
backward-compatible. A breaking change needs all of these:

- an accepted RFC (request for comments);
- a migration path;
- a deprecation period;
- compatibility fixtures;
- a new major-version directory.

The `tachyon-contracts` crate embeds these files. Its tests check every schema
against its meta-schema. They also run both examples through the compiled
validator.
