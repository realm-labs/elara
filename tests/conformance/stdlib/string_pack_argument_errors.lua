local missing_format_ok, missing_format_message = pcall(string.pack)
local format_type_ok, format_type_message = pcall(string.pack, false)
local missing_value_ok, missing_value_message = pcall(string.pack, "B")
local value_type_ok, value_type_message = pcall(string.pack, "B", false)

return missing_format_ok,
  string.byte(type(missing_format_message), 1),
  format_type_ok,
  string.byte(type(format_type_message), 1),
  missing_value_ok,
  string.byte(type(missing_value_message), 1),
  value_type_ok,
  string.byte(type(value_type_message), 1)
