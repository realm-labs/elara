local missing_format_ok, missing_format_message = pcall(string.unpack)
local format_type_ok, format_type_message = pcall(string.unpack, false, "")
local missing_data_ok, missing_data_message = pcall(string.unpack, "B")
local data_type_ok, data_type_message = pcall(string.unpack, "B", false)

return missing_format_ok,
  string.byte(type(missing_format_message), 1),
  format_type_ok,
  string.byte(type(format_type_message), 1),
  missing_data_ok,
  string.byte(type(missing_data_message), 1),
  data_type_ok,
  string.byte(type(data_type_message), 1)
