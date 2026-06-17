local char = string.char(65.0, "0x42")
local byte = string.byte("abc", "0x2")
local sub = string.sub("abcdef", "2.0", "0x1p2")
local repeated = string.rep("xy", "0x2", ".")
local found = string.find("abcabc", "a", "4")
local replaced, count = string.gsub("a1b2c3", "%d", "x", "2.0")
local bad_byte_ok, bad_byte_message = pcall(string.byte, "abc", "1.5")

return string.len(char), string.byte(char, 1), string.byte(char, 2),
  byte, string.len(sub), string.byte(sub, 1), string.byte(sub, 3),
  string.len(repeated), string.byte(repeated, 3), found,
  string.len(replaced), count, bad_byte_ok,
  string.byte(type(bad_byte_message), 1)
