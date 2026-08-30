---@meta

---@class Wasm_sql_2Dv02_connection
---@field ["query"] fun(self: Wasm_sql_2Dv02_connection, arg_1: string): table|nil, table|nil
---@field ["exec"] fun(self: Wasm_sql_2Dv02_connection, arg_1: string): table|nil, table|nil
---@field ["close"] fun(self: Wasm_sql_2Dv02_connection)

local M = {}

---@param arg_1 { ["endpoint"]: string, ["username-secret"]: string, ["password-secret"]: string, ["database"]: string|nil, ["max-rows"]: integer|nil, ["max-result-bytes"]: integer|nil }
---@return Wasm_sql_2Dv02_connection|nil
---@return table|nil
M["connect"] = function(arg_1) end

return M
