#!/usr/bin/env bash
set -euo pipefail

wasm-tools component wit wit/sigil-host/1.0.0/host.wit >/dev/null
wasm-tools component wit wit/sigil-host/1.1.0/host.wit >/dev/null
wasm-tools component wit wit/sigil-sql/0.1.0/sql.wit >/dev/null
wasm-tools component wit wit/sigil-sql/0.2.0/sql.wit >/dev/null

readonly SQL_V01_SHA256="1eed081d79af7aca61c343667b1a1d763b8be70f912d0c67f4b48d3ef0fee8f2"
readonly SQL_V01_BLAKE3="62cb190b6bcf7bf9fb45a974cb4865ec462c96520b4b88eef951c6b96774d068"
test "$(sha256sum wit/sigil-sql/0.1.0/sql.wit | awk '{print $1}')" = "$SQL_V01_SHA256"
test "$(b3sum wit/sigil-sql/0.1.0/sql.wit | awk '{print $1}')" = "$SQL_V01_BLAKE3"

sha256sum --check <(awk '$1 == "sha256" { print $2 "  " $3 }' WIT-DIGESTS)
b3sum --check <(awk '$1 == "blake3" { print $2 "  " $3 }' WIT-DIGESTS)
./scripts/check-sql-fixtures.sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
