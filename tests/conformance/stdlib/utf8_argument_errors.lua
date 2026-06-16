local char_type_ok, char_type_message = pcall(utf8.char, false)
local char_range_ok, char_range_message = pcall(utf8.char, -1)
local codepoint_string_ok, codepoint_string_message = pcall(utf8.codepoint, false)
local codepoint_start_ok, codepoint_start_message = pcall(utf8.codepoint, "abc", false)
local len_string_ok, len_string_message = pcall(utf8.len, false)
local offset_count_ok, offset_count_message = pcall(utf8.offset, "abc", false)

return char_type_ok,
  string.byte(type(char_type_message), 1),
  char_range_ok,
  string.byte(type(char_range_message), 1),
  codepoint_string_ok,
  string.byte(type(codepoint_string_message), 1),
  codepoint_start_ok,
  string.byte(type(codepoint_start_message), 1),
  len_string_ok,
  string.byte(type(len_string_message), 1),
  offset_count_ok,
  string.byte(type(offset_count_message), 1)
