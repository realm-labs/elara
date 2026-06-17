local char = utf8.char(65.0, "0x42")
local len = utf8.len("abc", "1.0", "0x1p1")
local cp1, cp2 = utf8.codepoint("abc", "1", "2.0")
local offset_start, offset_end = utf8.offset("abc", "0x2")
local bad_codepoint_ok, bad_codepoint_message =
  pcall(utf8.codepoint, "abc", "1.5")

return string.len(char), string.byte(char, 1), string.byte(char, 2),
  len, cp1, cp2, offset_start, offset_end, bad_codepoint_ok,
  string.byte(type(bad_codepoint_message), 1)
