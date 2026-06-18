local len_ok, len_message = pcall(string.len, nil)
local byte_value_ok, byte_value_message = pcall(string.byte, nil)
local char_ok, char_message = pcall(string.char, nil)
local lower_ok, lower_message = pcall(string.lower, nil)
local rep_value_ok, rep_value_message = pcall(string.rep, nil, 2)
local rep_count_ok, rep_count_message = pcall(string.rep, "x", nil)
local sub_value_ok, sub_value_message = pcall(string.sub, nil, 1)

return len_ok,
  string.byte(type(len_message), 1),
  byte_value_ok,
  string.byte(type(byte_value_message), 1),
  char_ok,
  string.byte(type(char_message), 1),
  lower_ok,
  string.byte(type(lower_message), 1),
  rep_value_ok,
  string.byte(type(rep_value_message), 1),
  rep_count_ok,
  string.byte(type(rep_count_message), 1),
  sub_value_ok,
  string.byte(type(sub_value_message), 1)
