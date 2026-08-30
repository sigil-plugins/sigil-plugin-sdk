#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/sigil-sql-conformance.XXXXXXXX")"
readonly SCRATCH
readonly LEFT="$SCRATCH/left"
readonly RIGHT="$SCRATCH/right"

cleanup() {
  rm -r -- "$SCRATCH"
}
trap cleanup EXIT

CARGO_TARGET_DIR="$LEFT" "$ROOT/scripts/check-sql-fixtures.sh" >/dev/null
CARGO_TARGET_DIR="$RIGHT" "$ROOT/scripts/check-sql-fixtures.sh" >/dev/null

cmp --silent \
  "$LEFT/sql-conformance/sql-v01.component.wasm" \
  "$RIGHT/sql-conformance/sql-v01.component.wasm"
cmp --silent \
  "$LEFT/sql-conformance/sql-v02.component.wasm" \
  "$RIGHT/sql-conformance/sql-v02.component.wasm"

sha256sum "$LEFT/sql-conformance/sql-v01.component.wasm" \
  "$LEFT/sql-conformance/sql-v02.component.wasm"
b3sum "$LEFT/sql-conformance/sql-v01.component.wasm" \
  "$LEFT/sql-conformance/sql-v02.component.wasm"
