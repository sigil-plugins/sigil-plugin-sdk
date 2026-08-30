---@meta

---@class Wasm_sql_2Dv02_connection
---@field ["query"] fun(self: Wasm_sql_2Dv02_connection, arg_1: string): { ["columns"]: { ["catalog"]: string, ["schema"]: string, ["table"]: string, ["original-table"]: string, ["name"]: string, ["original-name"]: string, ["vendor-type"]: integer, ["charset"]: integer, ["collation"]: integer, ["flags"]: integer, ["type"]: "null"|"signed"|"unsigned"|"floating"|"decimal"|"text"|"bytes"|"temporal", ["temporal-type"]: "date"|"time"|"time-with-time-zone"|"datetime"|"timestamp"|"timestamp-with-time-zone"|"interval"|"year"|"vendor"|nil }[], ["rows"]: { ["cells"]: table[] }[] }|nil, { ["class"]: "invalid"|"authentication"|"server"|"transport"|"protocol"|"encoding"|"limit"|"timeout"|"closed"|"unsupported", ["vendor-code"]: integer|nil, ["sqlstate"]: string|nil, ["message"]: string }|nil
---@field ["exec"] fun(self: Wasm_sql_2Dv02_connection, arg_1: string): { ["affected-rows"]: integer, ["last-insert-id"]: integer|nil, ["warnings"]: integer }|nil, { ["class"]: "invalid"|"authentication"|"server"|"transport"|"protocol"|"encoding"|"limit"|"timeout"|"closed"|"unsupported", ["vendor-code"]: integer|nil, ["sqlstate"]: string|nil, ["message"]: string }|nil
---@field ["close"] fun(self: Wasm_sql_2Dv02_connection)

local M = {}

---@param arg_1 { ["endpoint"]: string, ["username-secret"]: string, ["password-secret"]: string, ["database"]: string|nil, ["max-rows"]: integer|nil, ["max-result-bytes"]: integer|nil }
---@return Wasm_sql_2Dv02_connection|nil
---@return { ["class"]: "invalid"|"authentication"|"server"|"transport"|"protocol"|"encoding"|"limit"|"timeout"|"closed"|"unsupported", ["vendor-code"]: integer|nil, ["sqlstate"]: string|nil, ["message"]: string }|nil
M["connect"] = function(arg_1) end

return M
