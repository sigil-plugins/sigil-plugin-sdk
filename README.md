# Sigil Plugin SDK

Canonical WIT contracts for Sigil Component Model plugins.

- `wit/sigil-host/1.0.0/host.wit` is the byte-for-byte public copy of Sigil's private Host API 1.0 contract, including the additive `sigil:host/net-policy@1.0.0` interface.
- `wit/sigil-host/1.1.0/host.wit` adds only the opaque, read-only `sigil:host/sigv4@1.1.0` signed-exchange authority; it does not change or replace Host API 1.0.
- `wit/sigil-sql/0.1.0/sql.wit` remains the frozen canonical source of the experimental `sigil:sql/driver@0.1.0` export interface.
- `wit/sigil-sql/0.2.0/sql.wit` is the additive typed query/command contract. See [`docs/sql-0.2.0.md`](docs/sql-0.2.0.md) for exact value, bound, and migration semantics.

These contracts grant no ambient filesystem, process, stdio, clock, DNS, UDP, listen-socket, WASI, or arbitrary-network authority.

`sigil:host/net@1.0.0` remains the transport interface. The separate,
read-only `sigil:host/net-policy@1.0.0` interface exposes only the frozen
operator-selected TLS mode (`disabled`, `direct`, or `upgrade`) for a granted
logical endpoint through `get-tls-mode`. It does not reveal concrete routes,
hostnames, ports, DNS results, certificates, secrets, or additional network
authority.

## Layout

- `wit/sigil-host/1.0.0/` is the public byte-matched host contract.
- `wit/sigil-host/1.1.0/` is the nominally distinct opaque SigV4 exchange contract.
- `wit/sigil-sql/0.1.0/` is the frozen SQL 0.1 contract.
- `wit/sigil-sql/0.2.0/` is the nominally distinct typed SQL 0.2 contract.
- SQL consumers pin an immutable SDK revision and must not maintain divergent copies.
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
