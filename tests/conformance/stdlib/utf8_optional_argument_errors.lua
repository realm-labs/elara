local codepoint_end_ok, codepoint_end_message = pcall(utf8.codepoint, "abc", 1, false)
local len_end_ok, len_end_message = pcall(utf8.len, "abc", 1, false)
local offset_string_ok, offset_string_message = pcall(utf8.offset, false, 1)
local offset_position_ok, offset_position_message = pcall(utf8.offset, "abc", 1, false)

return codepoint_end_ok,
  string.byte(type(codepoint_end_message), 1),
  len_end_ok,
  string.byte(type(len_end_message), 1),
  offset_string_ok,
  string.byte(type(offset_string_message), 1),
  offset_position_ok,
  string.byte(type(offset_position_message), 1)
