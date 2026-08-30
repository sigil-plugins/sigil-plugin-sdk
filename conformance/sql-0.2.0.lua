-- Executable Lua 5.4 golden for Sigil's SQL 0.2 projection.
local fixture = {
  interface = "sigil:sql/driver@0.2.0",
  cells = {
    {tag = "null"},
    {tag = "signed", value = -9223372036854775807 - 1},
    {tag = "unsigned", value = 9223372036854775807},
    {tag = "unsigned", value_kind = "sigil.u64", decimal = "18446744073709551615"},
    {tag = "floating", value = -0.0},
    {tag = "decimal", value = "001.2300"},
    {tag = "text", value = "snowman ☃"},
    {tag = "bytes", value = "\0\255\128"},
    {tag = "temporal", value = "2026-08-30 12:34:56.000001+05:45"},
  },
  columns = {
    {name = "", type = "signed"},
    {name = "duplicate", type = "decimal"},
    {name = "duplicate", type = "temporal", ["temporal-type"] = "timestamp-with-time-zone"},
  },
  command = {
    ["affected-rows"] = "18446744073709551615",
    ["last-insert-id"] = 0,
    warnings = 2,
  },
  server_error = {
    class = "server",
    ["vendor-code"] = 1201,
    sqlstate = "HY000",
    message = "fixture rejection",
  },
}

assert(fixture.cells[1].tag == "null" and fixture.cells[1].value == nil)
assert(fixture.cells[2].value == -9223372036854775807 - 1)
assert(fixture.cells[4].value_kind == "sigil.u64")
assert(1 / fixture.cells[5].value == -math.huge)
assert(fixture.cells[6].value == "001.2300")
assert(#fixture.cells[8].value == 3)
assert(fixture.columns[2].name == fixture.columns[3].name)
assert(fixture.columns[3]["temporal-type"] == "timestamp-with-time-zone")
assert(fixture.command["last-insert-id"] == 0)
assert(fixture.server_error.sqlstate == "HY000")

return fixture
