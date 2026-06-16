local len_ok, len_message = pcall(string.len, false)
local byte_value_ok, byte_value_message = pcall(string.byte, false)
local byte_start_ok, byte_start_message = pcall(string.byte, "abc", false)
local char_ok, char_message = pcall(string.char, false)
local lower_ok, lower_message = pcall(string.lower, false)
local rep_count_ok, rep_count_message = pcall(string.rep, "x", false)
local rep_separator_ok, rep_separator_message = pcall(string.rep, "x", 2, false)
local sub_start_ok, sub_start_message = pcall(string.sub, "abc", false)
local find_pattern_ok, find_pattern_message = pcall(string.find, "abc", false)
local gsub_replacement_ok, gsub_replacement_message = pcall(string.gsub, "abc", "a", false)

return len_ok,
  string.byte(type(len_message), 1),
  byte_value_ok,
  string.byte(type(byte_value_message), 1),
  byte_start_ok,
  string.byte(type(byte_start_message), 1),
  char_ok,
  string.byte(type(char_message), 1),
  lower_ok,
  string.byte(type(lower_message), 1),
  rep_count_ok,
  string.byte(type(rep_count_message), 1),
  rep_separator_ok,
  string.byte(type(rep_separator_message), 1),
  sub_start_ok,
  string.byte(type(sub_start_message), 1),
  find_pattern_ok,
  string.byte(type(find_pattern_message), 1),
  gsub_replacement_ok,
  string.byte(type(gsub_replacement_message), 1)
