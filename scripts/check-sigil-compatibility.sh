#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 /path/to/sigil-binary" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT
SIGIL="$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")"
readonly SIGIL
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/sigil-sql-sdk.XXXXXXXX")"
readonly SCRATCH

cleanup() {
  rm -r -- "$SCRATCH"
}
trap cleanup EXIT

"$ROOT/scripts/check-sql-fixtures.sh" >/dev/null

for version in v01 v02; do
  mkdir -p "$SCRATCH/$version/dist"
  cp "$ROOT/fixtures/sql-$version/plugin.toml" "$SCRATCH/$version/plugin.toml"
  cp "$ROOT/target/sql-conformance/sql-$version.component.wasm" \
    "$SCRATCH/$version/plugin.wasm"
  "$SIGIL" plugin validate "$SCRATCH/$version/plugin.toml"
  "$SIGIL" plugin inspect "$SCRATCH/$version/plugin.toml" \
    >"$SCRATCH/$version/inspect.txt"
  grep -F "sigil:sql/driver@0.${version#v0}.0" \
    "$SCRATCH/$version/inspect.txt" >/dev/null
  grep -F "requested capabilities: none" "$SCRATCH/$version/inspect.txt" >/dev/null
  grep -F "imports: none" "$SCRATCH/$version/inspect.txt" >/dev/null
  "$SIGIL" plugin pack "$SCRATCH/$version/plugin.toml" \
    --output-dir "$SCRATCH/$version/dist"
done

if grep -F '[method]connection.exec' "$SCRATCH/v01/inspect.txt" >/dev/null; then
  echo "SQL 0.1 fixture unexpectedly reflected exec" >&2
  exit 1
fi
grep -F '[method]connection.exec' "$SCRATCH/v02/inspect.txt" >/dev/null

"$SIGIL" plugin validate \
  "$SCRATCH/v01/dist/sql-v01-0.0.1.sigil-plugin.tar.zst"
"$SIGIL" plugin validate \
  "$SCRATCH/v02/dist/sql-v02-0.0.1.sigil-plugin.tar.zst"
