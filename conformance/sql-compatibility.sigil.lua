local function connect_v01(sql_v01)
  local connection, err = sql_v01.connect({
    endpoint = "database",
    ["username-secret"] = "SQL_USER",
    ["password-secret"] = "SQL_PASSWORD",
  })
  expect(connection ~= nil, err and err.message)
  return connection
end

local function connect_v02(sql_v02, max_rows, max_result_bytes)
  local connection, err = sql_v02.connect({
    endpoint = "database",
    ["username-secret"] = "SQL_USER",
    ["password-secret"] = "SQL_PASSWORD",
    ["max-rows"] = max_rows,
    ["max-result-bytes"] = max_result_bytes,
  })
  expect(connection ~= nil, err and err.message)
  return connection
end

return {
  title = "SQL 0.1 and 0.2 remain nominal and lift exact values",
  priority = "P0",
  policy = { capabilities = { "wasm.sql-v01", "wasm.sql-v02" } },

  run = function()
    local sql_v01 = require("wasm.sql-v01")
    local sql_v02 = require("wasm.sql-v02")

    local old = connect_v01(sql_v01)
    local old_rows, old_err = old:query("SELECT lossless")
    expect(old_rows ~= nil, old_err and old_err.message)
    expect(old_rows.tag == "rows")
    expect(old_rows.value.rows[1].cells[1].tag == "null")
    expect(old_rows.value.rows[1].cells[1].value == nil)
    expect(old_rows.value.rows[1].cells[2].value == "snowman ☃")
    local old_b1, old_b2, old_b3 = string.byte(old_rows.value.rows[1].cells[3].value, 1, 3)
    expect(old_b1 == 0 and old_b2 == 255 and old_b3 == 128)
    local old_command, old_command_err = old:query("UPDATE fixture")
    expect(old_command ~= nil, old_command_err and old_command_err.message)
    expect(old_command.tag == "command")
    expect(old_command.value["affected-rows"] == 1)
    old:close()
    old:close()

    local typed = connect_v02(sql_v02)
    local rows, rows_err = typed:query("SELECT typed")
    expect(rows ~= nil, rows_err and rows_err.message)
    expect(#rows.columns == 8)
    expect(#rows.rows == 1)
    expect(#rows.rows[1].cells == #rows.columns)
    expect(rows.rows[1].cells[1].tag == "null")
    expect(rows.rows[1].cells[1].value == nil)
    expect(rows.rows[1].cells[2].value == -9223372036854775807 - 1)
    expect(type(rows.rows[1].cells[3].value) == "userdata")
    expect(tostring(rows.rows[1].cells[3].value) == "18446744073709551615")
    expect(1 / rows.rows[1].cells[4].value == -math.huge)
    expect(rows.rows[1].cells[5].value == "001.2300")
    expect(rows.rows[1].cells[6].value == "snowman ☃")
    local b1, b2, b3 = string.byte(rows.rows[1].cells[7].value, 1, 3)
    expect(b1 == 0 and b2 == 255 and b3 == 128)
    expect(rows.rows[1].cells[8].value == "2026-08-30 12:34:56.000001+05:45")
    expect(rows.columns[8].type == "temporal")
    expect(rows.columns[8]["temporal-type"] == "timestamp-with-time-zone")

    local maximum, maximum_err = typed:exec("COMMAND max")
    expect(maximum ~= nil, maximum_err and maximum_err.message)
    expect(type(maximum["affected-rows"]) == "userdata")
    expect(tostring(maximum["affected-rows"]) == "18446744073709551615")
    expect(type(maximum["last-insert-id"]) == "userdata")
    expect(tostring(maximum["last-insert-id"]) == "18446744073709551615")
    expect(maximum.warnings == 4294967295)

    local empty, empty_err = typed:query("SELECT empty")
    expect(empty ~= nil, empty_err and empty_err.message)
    expect(#empty.columns == 0)
    expect(#empty.rows == 0)

    local create, create_err = typed:exec("CREATE TEMPORARY TABLE conformance(value BIGINT)")
    expect(create ~= nil, create_err and create_err.message)
    expect(create["affected-rows"] == 0)
    expect(create["last-insert-id"] == nil)
    expect(create.warnings == 0)
    local insert, insert_err = typed:exec("INSERT INTO conformance VALUES (7)")
    expect(insert ~= nil, insert_err and insert_err.message)
    expect(insert["affected-rows"] == 1)
    expect(insert["last-insert-id"] == 0)
    expect(insert.warnings == 2)
    local temporary, temporary_err = typed:query("SELECT value FROM conformance")
    expect(temporary ~= nil, temporary_err and temporary_err.message)
    expect(temporary.rows[1].cells[1].value == 7)

    local wrong_query, wrong_query_err = typed:query("COMMAND")
    expect(wrong_query == nil)
    expect(wrong_query_err.class == "unsupported")
    local wrong_exec, wrong_exec_err = typed:exec("SELECT typed")
    expect(wrong_exec == nil)
    expect(wrong_exec_err.class == "unsupported")
    typed:close()
    typed:close()

    local exact_rows = connect_v02(sql_v02, 3, 48)
    local three, three_err = exact_rows:query("SELECT three")
    expect(three ~= nil, three_err and three_err.message)
    expect(#three.rows == 3)
    exact_rows:close()

    local too_many = connect_v02(sql_v02, 2)
    local limited_rows, row_limit = too_many:query("SELECT three")
    expect(limited_rows == nil)
    expect(row_limit.class == "limit")
    local after_row_limit, row_closed = too_many:query("SELECT three")
    expect(after_row_limit == nil)
    expect(row_closed.class == "closed")

    local too_many_bytes = connect_v02(sql_v02, nil, 47)
    local limited_bytes, byte_limit = too_many_bytes:query("SELECT three")
    expect(limited_bytes == nil)
    expect(byte_limit.class == "limit")

    local command_exact = connect_v02(sql_v02, nil, 20)
    local exact_command, exact_command_err = command_exact:exec("COMMAND max")
    expect(exact_command ~= nil, exact_command_err and exact_command_err.message)
    command_exact:close()

    local command_too_large = connect_v02(sql_v02, nil, 19)
    local limited_command, command_limit = command_too_large:exec("COMMAND max")
    expect(limited_command == nil)
    expect(command_limit.class == "limit")

    for _, case in ipairs({
      { statement = "ERROR encoding", class = "encoding" },
      { statement = "ERROR protocol", class = "protocol" },
      { statement = "ERROR timeout", class = "timeout" },
      { statement = "ERROR transport", class = "transport" },
    }) do
      local connection = connect_v02(sql_v02)
      local result, err = connection:query(case.statement)
      expect(result == nil)
      expect(err.class == case.class)
      local after, closed = connection:query("SELECT empty")
      expect(after == nil)
      expect(closed.class == "closed")
    end

    local server = connect_v02(sql_v02)
    local server_result, server_err = server:query("ERROR server")
    expect(server_result == nil)
    expect(server_err.class == "server")
    expect(server_err["vendor-code"] == 1201)
    expect(server_err.sqlstate == "HY000")
    local after_server, after_server_err = server:query("SELECT empty")
    expect(after_server ~= nil, after_server_err and after_server_err.message)
    server:close()
  end,
}
