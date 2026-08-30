#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 /path/to/sigil-binary [/path/to/sigil-checkout]" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly ROOT
SIGIL="$(cd "$(dirname "$1")" && pwd -P)/$(basename "$1")"
readonly SIGIL
if [[ ! -x "$SIGIL" ]]; then
  echo "sigil binary is not executable: $SIGIL" >&2
  exit 2
fi

infer_sigil_checkout() {
  local candidate
  candidate="$(dirname "$SIGIL")"
  while [[ "$candidate" != "/" ]]; do
    if [[ -f "$candidate/Cargo.toml" ]] &&
      grep -Eq '^name = "sigil"$' "$candidate/Cargo.toml"; then
      printf '%s\n' "$candidate"
      return 0
    fi
    candidate="$(dirname "$candidate")"
  done
  return 1
}

SIGIL_CHECKOUT="${2:-${SIGIL_CHECKOUT:-}}"
if [[ -z "$SIGIL_CHECKOUT" ]]; then
  SIGIL_CHECKOUT="$(infer_sigil_checkout || true)"
fi
if [[ -z "$SIGIL_CHECKOUT" || ! -f "$SIGIL_CHECKOUT/Cargo.toml" ]]; then
  echo "an exact Sigil source checkout is required for compatibility seeding and host-boundary tests" >&2
  exit 2
fi
SIGIL_CHECKOUT="$(cd "$SIGIL_CHECKOUT" && pwd -P)"
readonly SIGIL_CHECKOUT
if ! grep -Eq '^name = "sigil"$' "$SIGIL_CHECKOUT/Cargo.toml"; then
  echo "not a Sigil source checkout: $SIGIL_CHECKOUT" >&2
  exit 2
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/sigil-sql-sdk.XXXXXXXX")"
readonly SCRATCH
SOURCE="github:conformance/sql-fixtures"
readonly SOURCE

cleanup() {
  rm -r -- "$SCRATCH"
}
trap cleanup EXIT

run_sigil() {
  (
    cd "$SCRATCH/project"
    SIGIL_DATA_DIR="$SCRATCH/data" \
      SIGIL_CACHE_DIR="$SCRATCH/cache" \
      "$SIGIL" "$@"
  )
}

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

for version in v01 v02; do
  "$SIGIL" plugin validate \
    "$SCRATCH/$version/dist/sql-$version-0.0.1.sigil-plugin.tar.zst"
done

# Seed the scratch store with the exact package bytes as inactive remote
# acquisitions. Project locks intentionally cannot authorize local:path, and
# these fixtures are not published releases. This small checked helper uses
# Sigil's production PluginStore and the same third-party digest-only evidence
# shape used after a verified GitHub download; subsequent lock, stub, and run
# operations are all the unmodified Sigil CLI.
mkdir -p "$SCRATCH/seeder/src"
python3 - \
  "$ROOT/tools/sigil-compat-seed/Cargo.toml.in" \
  "$SIGIL_CHECKOUT" \
  "$SCRATCH/seeder/Cargo.toml" <<'PY'
from pathlib import Path
import sys

template = Path(sys.argv[1]).read_text(encoding="utf-8")
checkout = sys.argv[2].replace("\\", "\\\\").replace('"', '\\"')
Path(sys.argv[3]).write_text(
    template.replace("@SIGIL_CHECKOUT@", checkout), encoding="utf-8"
)
PY
cp "$ROOT/tools/sigil-compat-seed/main.rs" "$SCRATCH/seeder/src/main.rs"
cargo generate-lockfile --quiet --manifest-path "$SCRATCH/seeder/Cargo.toml" --offline
for version in v01 v02; do
  CARGO_TARGET_DIR="$ROOT/target/sigil-compat-seed" \
    cargo run --quiet --locked --offline \
      --manifest-path "$SCRATCH/seeder/Cargo.toml" -- \
      "$SCRATCH/data" \
      "$SCRATCH/$version/dist/sql-$version-0.0.1.sigil-plugin.tar.zst" \
      "$SOURCE" "sql-$version" 0.0.1 "sql-sdk-$version-0.0.1"
done

mkdir -p "$SCRATCH/project/.sigil" "$SCRATCH/project/scenarios"
cp "$ROOT/conformance/sql-compatibility.sigil.lua" \
  "$SCRATCH/project/scenarios/sql-compatibility.lua"
python3 - "$SCRATCH/project/.sigil/sigil.toml" "$SOURCE" <<'PY'
from pathlib import Path
import sys

Path(sys.argv[1]).write_text(
    "[deploy]\n"
    "backend = \"external\"\n"
    "\n"
    "[plugins]\n"
    "allow_third_party = true\n"
    "\n"
    "[plugins.trust]\n"
    f"install_allowlist = [\"{sys.argv[2]}\"]\n"
    "\n"
    "[plugins.require]\n"
    f"sql-v01 = {{ version = \"=0.0.1\", source = \"{sys.argv[2]}\" }}\n"
    f"sql-v02 = {{ version = \"=0.0.1\", source = \"{sys.argv[2]}\" }}\n",
    encoding="utf-8",
)
PY

run_sigil plugin list --format json >"$SCRATCH/installed.json"
python3 - "$SCRATCH/installed.json" "$SOURCE" <<'PY'
import json
from pathlib import Path
import sys

installed = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
actual = {
    (
        item["name"],
        item["version"],
        item["metadata"]["claimed_source"],
        item["metadata"]["acquisitions"][-1]["verification"],
    )
    for item in installed
}
expected = {
    ("sql-v01", "0.0.1", sys.argv[2], "third-party-digest-only"),
    ("sql-v02", "0.0.1", sys.argv[2], "third-party-digest-only"),
}
if actual != expected:
    raise SystemExit(f"installed SQL fixture identities differ: {actual!r}")
PY

run_sigil plugin lock
python3 - "$SCRATCH/project/.sigil/sigil.plugins.lock" "$SOURCE" <<'PY'
from pathlib import Path
import sys
import tomllib

lock = tomllib.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
actual = {
    (item["name"], item["version"], item["source"], item["verification"])
    for item in lock["plugin"]
}
expected = {
    ("sql-v01", "0.0.1", sys.argv[2], "third-party-digest-only"),
    ("sql-v02", "0.0.1", sys.argv[2], "third-party-digest-only"),
}
if actual != expected:
    raise SystemExit(f"locked SQL fixture identities differ: {actual!r}")
PY

V01_STUB="$SCRATCH/project/.sigil/types/wasm/sql-v01.lua"
V02_STUB="$SCRATCH/project/.sigil/types/wasm/sql-v02.lua"
readonly V01_STUB V02_STUB
test -f "$V01_STUB"
test -f "$V02_STUB"
if grep -F '["exec"]' "$V01_STUB" >/dev/null; then
  echo "SQL 0.1 generated stub unexpectedly exposes exec" >&2
  exit 1
fi
grep -F '["exec"]' "$V02_STUB" >/dev/null
diff -u "$ROOT/conformance/sql-0.2.0.stub.lua" "$V02_STUB"
sha256sum "$V01_STUB" "$V02_STUB" >"$SCRATCH/stubs.before"
run_sigil generate-types >/dev/null
sha256sum "$V01_STUB" "$V02_STUB" >"$SCRATCH/stubs.after"
cmp "$SCRATCH/stubs.before" "$SCRATCH/stubs.after"

# This is the production component loader and Lua bridge, not a table-shaped
# mock: both nominal modules are required from the exact project lock and their
# real resource methods return lifted NULL, bytes, u64, row, and command values.
if ! run_sigil run scenarios/sql-compatibility.lua --json >"$SCRATCH/run.json"; then
  python3 -m json.tool "$SCRATCH/run.json" >&2
  exit 1
fi
python3 - "$SCRATCH/run.json" <<'PY'
import json
from pathlib import Path
import sys

report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
if report["status"] != "passed" or report["total"] != 1 or report["failed"] != 0:
    raise SystemExit(f"SQL compatibility scenario failed: {report!r}")
PY

# Safe WIT guest bindings cannot forge an unknown variant discriminant or an
# invalid canonical string: canonical ABI validation rejects those values
# before a SQL error value could exist. Sigil's hand-built malicious components
# exercise that actual component/host boundary, including exact list ceilings.
cargo test --quiet --locked --manifest-path "$SIGIL_CHECKOUT/Cargo.toml" \
  --test plugin_fixture_components malformed_value_exports_fail_deterministically
for test_name in \
  invalid_sum_abi_is_typed_and_latched_across_caught_and_uncaught_lua_errors \
  malformed_canonical_strings_fail_in_wasmtime_before_lossy_conversion \
  element_depth_and_allocation_limits_accept_exact_boundary_then_fail_plus_one; do
  cargo test --quiet --locked --manifest-path "$SIGIL_CHECKOUT/Cargo.toml" \
    --lib "$test_name"
done
