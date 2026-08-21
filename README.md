# Sigil Plugin SDK

Canonical WIT contracts for Sigil Component Model plugins.

- `wit/sigil-host/1.0.0/host.wit` is the byte-for-byte public copy of Sigil's private Host API 1.0 contract.
- `wit/sigil-sql/0.1.0/sql.wit` is the sole canonical source of the experimental `sigil:sql/driver@0.1.0` export interface.

These contracts grant no ambient filesystem, process, stdio, clock, DNS, UDP, listen-socket, WASI, or arbitrary-network authority.
