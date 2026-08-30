#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT
readonly TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
readonly CARGO_HOME_PATH="${CARGO_HOME:-$HOME/.cargo}"
readonly ORIGINAL_RUSTFLAGS="${RUSTFLAGS:-}"
readonly REMAPPED_RUSTFLAGS="${ORIGINAL_RUSTFLAGS} --remap-path-prefix=${ROOT}=/workspace --remap-path-prefix=${CARGO_HOME_PATH}=/cargo"

cmp --silent \
  "$ROOT/wit/sigil-sql/0.1.0/sql.wit" \
  "$ROOT/fixtures/sql-v01/wit/deps/sigil-sql/sql.wit"
cmp --silent \
  "$ROOT/wit/sigil-sql/0.2.0/sql.wit" \
  "$ROOT/fixtures/sql-v02/wit/deps/sigil-sql/sql.wit"

RUSTFLAGS="${REMAPPED_RUSTFLAGS# }" cargo build \
  --manifest-path "$ROOT/Cargo.toml" \
  --release \
  --target wasm32-unknown-unknown \
  --locked \
  -p sigil-sql-v01-conformance-component \
  -p sigil-sql-v02-conformance-component

mkdir -p "$TARGET/sql-conformance"

(
  cd "$ROOT/fixtures/sql-v01"
  wasm-tools component new \
    "$TARGET/wasm32-unknown-unknown/release/sigil_sql_v01_conformance_component.wasm" \
    -o "$TARGET/sql-conformance/sql-v01.component.wasm"
)
(
  cd "$ROOT/fixtures/sql-v02"
  wasm-tools component new \
    "$TARGET/wasm32-unknown-unknown/release/sigil_sql_v02_conformance_component.wasm" \
    -o "$TARGET/sql-conformance/sql-v02.component.wasm"
)

wasm-tools validate --features all "$TARGET/sql-conformance/sql-v01.component.wasm"
wasm-tools validate --features all "$TARGET/sql-conformance/sql-v02.component.wasm"
(
  cd "$ROOT/fixtures/sql-v01"
  wasm-tools component targets wit \
    --world sigil:sql-conformance-v01/fixture@0.0.0 \
    "$TARGET/sql-conformance/sql-v01.component.wasm"
)
(
  cd "$ROOT/fixtures/sql-v02"
  wasm-tools component targets wit \
    --world sigil:sql-conformance-v02/fixture@0.0.0 \
    "$TARGET/sql-conformance/sql-v02.component.wasm"
)

wasm-tools component wit "$TARGET/sql-conformance/sql-v01.component.wasm" \
  | grep -F 'export sigil:sql/driver@0.1.0;' >/dev/null
wasm-tools component wit "$TARGET/sql-conformance/sql-v02.component.wasm" \
  | grep -F 'export sigil:sql/driver@0.2.0;' >/dev/null

if wasm-tools component wit "$TARGET/sql-conformance/sql-v01.component.wasm" \
  | grep -F 'export sigil:sql/driver@0.2.0;' >/dev/null; then
  echo "SQL 0.1 fixture unexpectedly exports SQL 0.2" >&2
  exit 1
fi
if wasm-tools component wit "$TARGET/sql-conformance/sql-v02.component.wasm" \
  | grep -F 'export sigil:sql/driver@0.1.0;' >/dev/null; then
  echo "SQL 0.2 fixture unexpectedly exports SQL 0.1" >&2
  exit 1
fi

sha256sum "$TARGET/sql-conformance/sql-v01.component.wasm" \
  "$TARGET/sql-conformance/sql-v02.component.wasm"
b3sum "$TARGET/sql-conformance/sql-v01.component.wasm" \
  "$TARGET/sql-conformance/sql-v02.component.wasm"
