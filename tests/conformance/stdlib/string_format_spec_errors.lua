local string_flag_ok, string_flag_message = pcall(string.format, "%+s", "x")
local pointer_precision_ok, pointer_precision_message = pcall(string.format, "%.3p", nil)
local char_flag_ok, char_flag_message = pcall(string.format, "%#c", 65)
local wide_width_ok, wide_width_message = pcall(string.format, "%999s", "x")
local quote_width_ok, quote_width_message = pcall(string.format, "%10q", "x")

return string_flag_ok,
  string.byte(type(string_flag_message), 1),
  pointer_precision_ok,
  string.byte(type(pointer_precision_message), 1),
  char_flag_ok,
  string.byte(type(char_flag_message), 1),
  wide_width_ok,
  string.byte(type(wide_width_message), 1),
  quote_width_ok,
  string.byte(type(quote_width_message), 1)
