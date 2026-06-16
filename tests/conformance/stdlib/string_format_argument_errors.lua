local format_ok, format_message = pcall(string.format, false)
local missing_string_ok, missing_string_message = pcall(string.format, "%s")
local integer_ok, integer_message = pcall(string.format, "%d", false)
local float_ok, float_message = pcall(string.format, "%f", false)
local char_ok, char_message = pcall(string.format, "%c", false)
local quote_ok, quote_message = pcall(string.format, "%q", {})

return format_ok,
  string.byte(type(format_message), 1),
  missing_string_ok,
  string.byte(type(missing_string_message), 1),
  integer_ok,
  string.byte(type(integer_message), 1),
  float_ok,
  string.byte(type(float_message), 1),
  char_ok,
  string.byte(type(char_message), 1),
  quote_ok,
  string.byte(type(quote_message), 1)
