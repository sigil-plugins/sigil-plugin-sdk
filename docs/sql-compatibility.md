# SQL interface compatibility matrix

SQL 0.1 and 0.2 are separate nominal Component Model interfaces. They can be
present in the same plugin cache, but one project requirement and lock select
one exact plugin package and its manifest selects one exact entrypoint.

| Contract | Entrypoint | Query result | Command call | Cell tags |
|---|---|---|---|---|
| frozen 0.1 | `sigil:sql/driver@0.1.0` | `rows` or `command` | none | `null`, `text`, `bytes` |
| experimental 0.2 | `sigil:sql/driver@0.2.0` | rows only | `exec` | `null`, `signed`, `unsigned`, `floating`, `decimal`, `text`, `bytes`, `temporal` |

No lock, generated stub, or `require("wasm.NAME")` module may substitute one
entrypoint for the other. An implementation migrating to 0.2 publishes a new
plugin version, vendors the 0.2 WIT package, and keeps the immutable 0.1
artifact available under its original digest. Existing 0.1 consumers do not
gain typed cells or `exec` through a source-compatible alias.

The conformance kit pins this distinction four ways:

1. `scripts/check-contracts.sh` hard-codes the frozen 0.1 SHA-256 and BLAKE3.
2. `conformance/sql-0.2.0.json` names both exact entrypoints and marks them
   nominally incompatible.
3. The fixture components build and validate against separate byte-matched WIT
   dependencies.
4. `scripts/check-sigil-compatibility.sh` seeds separate remote-style store
   identities, creates one exact project lock, compares the generated 0.2 stub
   to its golden, proves the 0.1 stub lacks `exec`, and requires both modules in
   one real Sigil scenario.

The fixture packages are test inputs, not releases. They grant no host imports,
open no network connection, read no secret, and must never be published as SQL
drivers.
