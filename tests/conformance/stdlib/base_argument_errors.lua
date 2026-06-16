local rawget_ok, rawget_message = pcall(rawget, false, "key")
local rawlen_ok, rawlen_message = pcall(rawlen, false)
local rawset_ok, rawset_message = pcall(rawset, false, "key", 1)
local select_ok, select_message = pcall(select, 0, "value")
local setmetatable_ok, setmetatable_message = pcall(setmetatable, false, {})
local tonumber_ok, tonumber_message = pcall(tonumber, "10", 1)
local warn_ok, warn_message = pcall(warn, false)

return rawget_ok,
  string.byte(type(rawget_message), 1),
  rawlen_ok,
  string.byte(type(rawlen_message), 1),
  rawset_ok,
  string.byte(type(rawset_message), 1),
  select_ok,
  string.byte(type(select_message), 1),
  setmetatable_ok,
  string.byte(type(setmetatable_message), 1),
  tonumber_ok,
  string.byte(type(tonumber_message), 1),
  warn_ok,
  string.byte(type(warn_message), 1)
