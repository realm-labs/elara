local missing_ok, missing_message = pcall(string.packsize)
local type_ok, type_message = pcall(string.packsize, false)

return missing_ok,
  string.byte(type(missing_message), 1),
  type_ok,
  string.byte(type(type_message), 1)
