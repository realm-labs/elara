local rawequal_ok, rawequal_message = pcall(rawequal)
local rawget_ok, rawget_message = pcall(rawget, {})
local rawset_ok, rawset_message = pcall(rawset, {}, "key")
local next_ok, next_message = pcall(next, false)
local getmetatable_ok, getmetatable_message = pcall(getmetatable)
local select_type_ok, select_type_message = pcall(select, false, "value")
local tostring_ok, tostring_message = pcall(tostring)
local type_ok, type_message = pcall(type)

return rawequal_ok,
  string.byte(type(rawequal_message), 1),
  rawget_ok,
  string.byte(type(rawget_message), 1),
  rawset_ok,
  string.byte(type(rawset_message), 1),
  next_ok,
  string.byte(type(next_message), 1),
  getmetatable_ok,
  string.byte(type(getmetatable_message), 1),
  select_type_ok,
  string.byte(type(select_type_message), 1),
  tostring_ok,
  string.byte(type(tostring_message), 1),
  type_ok,
  string.byte(type(type_message), 1)
