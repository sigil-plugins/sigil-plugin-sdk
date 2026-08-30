# SQL driver 0.2 contract

`sigil:sql/driver@0.2.0` is an additive experimental interface. It does not
replace or mutate `sigil:sql/driver@0.1.0`. A manifest selects one exact
nominal interface.

## Values and Lua projection

Every cell is a WIT variant. Sigil projects a variant to a Lua table with an
exact `tag` and an optional `value` field.

| SQL value | WIT cell | Lua shape |
|---|---|---|
| `NULL` | `null` | `{tag = "null"}` |
| signed integer | `signed(s64)` | `{tag = "signed", value = integer}` |
| unsigned integer | `unsigned(u64)` | `{tag = "unsigned", value = integer-or-sigil-u64}` |
| approximate number | `floating(f64)` | `{tag = "floating", value = number}` |
| exact numeric | `decimal(string)` | `{tag = "decimal", value = exact_text}` |
| character value | `text(string)` | `{tag = "text", value = text}` |
| binary value | `bytes(list<u8>)` | `{tag = "bytes", value = byte_string}` |
| date or time value | `temporal(string)` | `{tag = "temporal", value = exact_text}` |

`NULL` is never Lua `nil`. This rule preserves the position of every cell.
Lua `nil` represents an absent WIT option, such as `last-insert-id`. A result
returns `(value, nil)` on success or `(nil, error)` on failure.

Lua integers cover WIT `s64`. A WIT `u64` through `i64::MAX` is a Lua integer.
A larger `u64` is Sigil's exact `sigil.u64` userdata. It supports decimal
`tostring`, equality, and ordering with another `sigil.u64`. It never becomes
a floating-point number.

`decimal` preserves its UTF-8 server lexeme exactly, including sign, zeroes,
decimal point, exponent spelling, and vendor-defined special values. A driver
must not parse it through a floating type. `floating` is an intentional
conversion to IEEE 754 binary64. It preserves negative zero, infinities, and
NaN semantics, but it does not promise to preserve the source text or NaN
payload.

`temporal` preserves its UTF-8 server lexeme exactly. The matching ordered
column has `type = "temporal"` and a non-nil `temporal-type`. This metadata
distinguishes date, time, time with time zone, datetime, timestamp, timestamp
with time zone, interval, year, and a documented vendor type. The driver must
not change units, time zones, precision, offsets, or spelling.

Columns and cells are positional lists. For each row, `#cells == #columns`,
and cell `i` belongs to column `i`. A non-null cell tag must match the column
`type`. A temporal column must have `temporal-type`; other columns must not.
This shape preserves duplicate and empty column labels.

## Query, command, and session semantics

`query(sql)` accepts one statement and returns one row set. An OK or command
response is `unsupported`; the driver must not manufacture an empty row set.
`exec(sql)` accepts one statement and returns one command result. A row set is
`unsupported`; the driver must not discard rows.

`warnings` is the protocol warning count. It is not a list of warning text.
A caller can issue a later vendor statement such as `SHOW WARNINGS` on the
same connection when it needs details. `last-insert-id` is absent when the
protocol has no meaningful value. A numeric zero remains present when the
protocol explicitly reports zero.

One `connection` resource is one stateful server session. All calls observe
the same session state, including transactions and temporary tables. `close`
is idempotent. A method after close returns `closed`. A terminal protocol,
encoding, transport, timeout, or limit error closes the session unless the
driver can prove the protocol is synchronized. The driver never reconnects,
retries, or replays a statement.

Multi-statements, multi-results, prepared statements, binary result rows,
implicit reconnect, connection pooling, local file loading, and hidden retry
are outside this version. A driver must return `unsupported` before it exposes
a partial or ambiguous result.

## Bounds and deadlines

`max-rows` and `max-result-bytes` are caller-selected additional ceilings for
the connection. A missing field selects no caller ceiling. A present zero is
a real zero ceiling. The effective value is the minimum of the caller value,
the driver's fixed safety maximum, and any operator-owned semantic maximum.
A large caller value cannot increase authority. Both ceilings apply anew to
each `query` or `exec` result; they are not budgets shared across calls.

`max-result-bytes` uses one portable logical-byte count. Count the UTF-8 byte
length of each column's catalog, schema, table, original-table, name, and
original-name. Count eight bytes for each signed, unsigned, or
floating cell. Count the payload length of each decimal, text, bytes, or
temporal cell. Count zero for a null cell. A command counts eight bytes for
`affected-rows`, eight more when `last-insert-id` is present, and four for
`warnings`. Do not count tags, list lengths, pointers, capacity, alignment, or
allocator overhead. Drivers must use checked arithmetic before allocation. A
count overflow, maximum plus one, or either result ceiling returns `limit`,
discards the whole result, and never returns a short row set that looks
complete.

The endpoint network grant independently owns the aggregate wire `max_bytes`
and its connect, read, write, and outer-call deadlines. Host failures at those
ceilings remain authoritative. SQL 0.2 has no caller timeout field because
`sigil:host/net@1.0.0` has no scoped operation that can lower a host deadline.
A plugin has no clock and cannot implement such an option honestly. A future
version may add a caller deadline only with an additive host ABI that creates
a bounded stream or operation whose effective deadline is:

```text
min(caller request, operator grant deadline, outer call deadline)
```

A deadline error returned by a network operation maps to `timeout`. An outer
call deadline or cancellation returns no guest value and remains plugin
infrastructure. Transport loss maps to `transport`. A driver must not relabel
an operator denial or infrastructure fault as target behavior.

## Errors

The driver returns no partial result with an error.

| Condition | Class |
|---|---|
| Invalid caller option or empty required argument | `invalid` |
| Credential rejection | `authentication` |
| SQL or vendor rejection | `server` |
| Ordinary connection or socket loss | `transport` |
| Malformed or contradictory protocol data | `protocol` |
| Invalid text or numeric wire lexeme | `encoding` |
| Checked size overflow or a reached ceiling | `limit` |
| Host-reported deadline expiry | `timeout` |
| Use after close or terminal failure | `closed` |
| Valid feature or representation outside this contract | `unsupported` |

For a server error, retain a bounded `vendor-code` and validated five-byte
`sqlstate` when the server supplied them. The message must name an invalid
argument when the caller caused the error. It must remain bounded, escaped,
and secret-redacted. A valid numeric lexeme outside its WIT integer range is
`unsupported`. An internal size accumulation overflow is `limit`.

## MySQL text-protocol mapping

The MySQL type number is retained as `vendor-type`. Column flags retain the
server flags, including `UNSIGNED_FLAG`. The mapping below covers every value
in MySQL's published `enum_field_types`. The official definitions are in
[mysql_field_types_bits.h](https://dev.mysql.com/doc/dev/mysql-server/latest/mysql__field__types__bits_8h.html).
The text protocol sends non-null fields as length-encoded byte strings, per
the official [text row](https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_com_query_response_text_resultset_row.html)
definition.

| Code | MySQL type | SQL 0.2 mapping |
|---:|---|---|
| 0, 246 | `DECIMAL`, `NEWDECIMAL` | `decimal`, exact ASCII lexeme |
| 1, 2, 3, 8, 9 | `TINY`, `SHORT`, `LONG`, `LONGLONG`, `INT24` | `signed` or `unsigned` from `UNSIGNED_FLAG` |
| 4, 5 | `FLOAT`, `DOUBLE` | `floating`; invalid text is `encoding`, finite overflow is `unsupported` |
| 6 | `NULL` | column type `null`; every cell is `null` |
| 7, 17 | `TIMESTAMP`, `TIMESTAMP2` | `temporal`, type `timestamp`, exact lexeme |
| 10 | `DATE` | `temporal`, type `date`, exact lexeme |
| 11, 19 | `TIME`, `TIME2` | `temporal`, type `time`, exact lexeme |
| 12, 18 | `DATETIME`, `DATETIME2` | `temporal`, type `datetime`, exact lexeme |
| 13 | `YEAR` | `temporal`, type `year`, exact lexeme |
| 15 | `VARCHAR` | `text` for a supported text collation; otherwise lossless `bytes` |
| 16 | `BIT` | `bytes`; do not coerce bit width into an integer |
| 20 | `TYPED_ARRAY` | `unsupported`; replication-only in MySQL |
| 242 | `VECTOR` | `unsupported` until a lossless shared representation exists |
| 243 | `INVALID` | `protocol` |
| 244 | `BOOL` | `unsupported`; MySQL documents it as a placeholder |
| 245 | `JSON` | `text` after strict UTF-8 validation |
| 247, 248 | `ENUM`, `SET` | `text` for a supported text collation; otherwise lossless `bytes` |
| 249-252 | `TINY_BLOB`, `MEDIUM_BLOB`, `LONG_BLOB`, `BLOB` | `bytes` for binary collation; strict UTF-8 `text` otherwise |
| 253, 254 | `VAR_STRING`, `STRING` | `text` for a supported text collation; otherwise lossless `bytes` |
| 255 | `GEOMETRY` | `bytes` |
| 14, 21-241 except 242 | `NEWDATE` or unassigned | `unsupported` |

The row `NULL` marker takes precedence over the declared column type. Integer
parsing accepts only the complete base-10 wire lexeme and checks its target
range. A boolean alias transmitted as `TINY` remains numeric. String mapping
uses collation metadata; it never guesses from byte content alone.

The shared shape also permits a future PostgreSQL text-protocol driver. It can
retain the type OID in `vendor-type`, map signed integer OIDs to `signed`, map
`numeric` to exact `decimal`, map `bytea` to `bytes`, and preserve all date and
time output in `temporal`. PostgreSQL output can itself normalize a stored
value; the plugin must preserve exactly what the server returned and perform
no second normalization.

## Migration and rejected alternatives

SQL 0.1 and 0.2 are nominally distinct. A 0.1 plugin continues to export
`sigil:sql/driver@0.1.0`. A 0.2 plugin vendors the new package and exports
`sigil:sql/driver@0.2.0`. Locks, manifests, generated bindings, and host
reflection must never treat one as the other.

| SQL 0.1 | SQL 0.2 migration |
|---|---|
| `connect-options` has four fields | Add optional `max-rows` and `max-result-bytes` ceilings |
| `cell` has `null`, `text`, and `bytes` | Handle all eight tags exhaustively; do not infer a tag from Lua value type |
| `column` has vendor metadata | Also emit `type` and the conditional `temporal-type` |
| `query` returns `rows` or `command` | Use row-only `query`; use `exec` for a command |
| command has `affected-rows` | Also preserve optional `last-insert-id` and the warning count |
| one connection resource | Keep one session and the same idempotent-close rule |

This contract rejects these alternatives:

- Changing 0.1 in place would invalidate immutable consumers.
- Lua `nil` for SQL `NULL` would erase positional cells and conflate absence.
- Floating DECIMAL would lose exact precision and scale spelling.
- Normalized timestamps would hide unit, precision, offset, and timezone bugs.
- Row tables keyed by labels would lose duplicate or empty column names.
- A combined query-or-command variant would let callers ignore the wrong arm.
- Detailed warning lists would require hidden follow-up SQL and alter session state.
- Caller timeout fields without a host deadline ABI would be decorative.
- Retry, reconnect, replay, and multi-result handling would hide failures or duplicate effects.
