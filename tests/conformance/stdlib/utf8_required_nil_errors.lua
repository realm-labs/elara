local char_ok, char_message = pcall(utf8.char, nil)
local codepoint_ok, codepoint_message = pcall(utf8.codepoint, nil)
local codes_ok, codes_message = pcall(utf8.codes, nil)
local len_ok, len_message = pcall(utf8.len, nil)
local offset_string_ok, offset_string_message = pcall(utf8.offset, nil, 1)
local offset_count_ok, offset_count_message = pcall(utf8.offset, "abc", nil)

return char_ok,
  string.byte(type(char_message), 1),
  codepoint_ok,
  string.byte(type(codepoint_message), 1),
  codes_ok,
  string.byte(type(codes_message), 1),
  len_ok,
  string.byte(type(len_message), 1),
  offset_string_ok,
  string.byte(type(offset_string_message), 1),
  offset_count_ok,
  string.byte(type(offset_count_message), 1)
