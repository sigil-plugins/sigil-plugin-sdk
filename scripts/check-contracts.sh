#!/usr/bin/env bash
set -euo pipefail

wasm-tools component wit wit/sigil-host/1.0.0/host.wit >/dev/null
wasm-tools component wit wit/sigil-sql/0.1.0/sql.wit >/dev/null
sha256sum --check <(awk '$1 == "sha256" { print $2 "  " $3 }' WIT-DIGESTS)
b3sum --check <(awk '$1 == "blake3" { print $2 "  " $3 }' WIT-DIGESTS)
cargo fmt --all -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
