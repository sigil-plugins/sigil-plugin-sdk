# Sigil Plugin SDK

Canonical WIT contracts for Sigil Component Model plugins.

- `wit/sigil-host/1.0.0/host.wit` is the byte-for-byte public copy of Sigil's private Host API 1.0 contract.
- `wit/sigil-sql/0.1.0/sql.wit` is the sole canonical source of the experimental `sigil:sql/driver@0.1.0` export interface.

These contracts grant no ambient filesystem, process, stdio, clock, DNS, UDP, listen-socket, WASI, or arbitrary-network authority.

## Layout

- `wit/sigil-host/1.0.0/` is the public byte-matched host contract.
- `wit/sigil-sql/0.1.0/` is the only canonical SQL 0.1 contract; consumers pin an immutable SDK revision and must not maintain copies.
- `src/` provides reference Rust models and a deterministic authority-free test host.
- `conformance/` contains language-neutral boundary and error-mapping vectors.

Run `just check` without a Sigil checkout. Host maintainers additionally run
`just drift /path/to/sigil` before publishing a host API change. The SDK is not
a plugin and deliberately does not carry the `sigil-plugin` GitHub topic.

## Release synchronization

1. Freeze and review the canonical private host WIT.
2. Copy it byte-for-byte to the matching versioned SDK path.
3. Run the drift and standalone conformance checks.
4. Publish and read back the SDK commit before releasing a Sigil host that advertises that contract.

Host compatibility is defined by WIT, never by this Rust convenience crate.
