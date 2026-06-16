local codepoint_start_ok, codepoint_start_message = pcall(utf8.codepoint, "abc", 0)
local codepoint_end_ok, codepoint_end_message = pcall(utf8.codepoint, "abc", 1, 4)
local len_start_ok, len_start_message = pcall(utf8.len, "abc", 0)
local len_end_ok, len_end_message = pcall(utf8.len, "abc", 1, 4)
local offset_position_ok, offset_position_message = pcall(utf8.offset, "abc", 1, 0)

return codepoint_start_ok,
  string.byte(type(codepoint_start_message), 1),
  codepoint_end_ok,
  string.byte(type(codepoint_end_message), 1),
  len_start_ok,
  string.byte(type(len_start_message), 1),
  len_end_ok,
  string.byte(type(len_end_message), 1),
  offset_position_ok,
  string.byte(type(offset_position_message), 1)
