# SQL conformance boundary coverage

The SQL 0.2 corpus distinguishes malformed database data from malformed
Component Model values. They do not have the same observable error surface.

## Values a driver can reject

Row-width mismatch, a temporal column without `temporal-type`, and temporal
metadata on a non-temporal column are valid WIT values but invalid SQL 0.2 row
sets. The SDK reference validator constructs each shape and returns `protocol`.
The fixture scenario also sends complete row and command results at the exact
caller ceiling and one unit beyond it; the oversized call returns terminal
`limit` with no partial result.

Logical-byte overflow is executable only in the checked accounting primitive.
Constructing enough real strings or lists to overflow `u64` would require more
than 16 EiB and is impossible under Sigil's much smaller component allocation
ceiling. The test therefore drives `checked_add(u64::MAX, 1)` directly and
expects `limit`; it does not claim that such an allocation crossed Wasmtime.

## Values only a malicious component can attempt

Safe WIT bindings cannot construct an unknown `cell` discriminant or an invalid
UTF-8 WIT `string`. Those values cannot become a guest-returned SQL `error`:
the Canonical ABI rejects them while lifting the result. The Sigil integration
gate therefore runs hand-built malicious-component tests from the exact Sigil
checkout. They verify invalid sum discriminants are typed and latch the plugin
instance, and malformed canonical strings fail in Wasmtime before any lossy
conversion. The host's exact element/depth/allocation boundary test separately
accepts the maximum and rejects maximum plus one.

These host failures are plugin infrastructure failures, not target-database
`protocol` or `encoding` errors. A driver uses those SQL classes only for
malformed wire data it successfully received as valid canonical bytes.

## Real compatibility path

`scripts/check-sigil-compatibility.sh` packs the two deterministic fixture
components and seeds an isolated Sigil plugin store using remote-style
`third-party-digest-only` acquisitions. This is a test-only substitute for a
GitHub download because the fixtures are intentionally not published. The
unmodified Sigil CLI then resolves both project requirements, writes an exact
lock and managed stubs, regenerates those stubs byte-identically, and runs
`conformance/sql-compatibility.sigil.lua`.

That scenario's tables are actual values lifted by Sigil from the fixture
components. It checks the nominally distinct 0.1 result variant and 0.2
row-only/`exec` surfaces, resource lifecycle, positional NULL, invalid UTF-8
bytes as a Lua byte string, `u64::MAX` userdata, exact DECIMAL and temporal
strings, command metadata, stateful session calls, and terminal classes.
