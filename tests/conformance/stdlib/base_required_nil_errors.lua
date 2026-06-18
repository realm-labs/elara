local rawget_ok, rawget_message = pcall(rawget, nil, "key")
local rawlen_ok, rawlen_message = pcall(rawlen, nil)
local rawset_ok, rawset_message = pcall(rawset, nil, "key", 1)
local next_ok, next_message = pcall(next, nil)
local setmetatable_ok, setmetatable_message = pcall(setmetatable, nil, {})
local select_ok, select_message = pcall(select, nil, "value")

return rawget_ok,
  string.byte(type(rawget_message), 1),
  rawlen_ok,
  string.byte(type(rawlen_message), 1),
  rawset_ok,
  string.byte(type(rawset_message), 1),
  next_ok,
  string.byte(type(next_message), 1),
  setmetatable_ok,
  string.byte(type(setmetatable_message), 1),
  select_ok,
  string.byte(type(select_message), 1)
