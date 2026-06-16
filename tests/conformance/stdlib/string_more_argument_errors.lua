local upper_ok, upper_message = pcall(string.upper, false)
local reverse_ok, reverse_message = pcall(string.reverse, false)
local byte_end_ok, byte_end_message = pcall(string.byte, "abc", 1, false)
local sub_end_ok, sub_end_message = pcall(string.sub, "abc", 1, false)
local find_subject_ok, find_subject_message = pcall(string.find, false, "a")
local find_init_ok, find_init_message = pcall(string.find, "abc", "a", false)
local match_subject_ok, match_subject_message = pcall(string.match, false, "a")
local match_pattern_ok, match_pattern_message = pcall(string.match, "abc", false)
local match_init_ok, match_init_message = pcall(string.match, "abc", "a", false)
local gmatch_subject_ok, gmatch_subject_message = pcall(string.gmatch, false, "a")
local gmatch_pattern_ok, gmatch_pattern_message = pcall(string.gmatch, "abc", false)
local gsub_subject_ok, gsub_subject_message = pcall(string.gsub, false, "a", "b")
local gsub_pattern_ok, gsub_pattern_message = pcall(string.gsub, "abc", false, "b")
local gsub_replacement_ok, gsub_replacement_message = pcall(string.gsub, "abc", "a")
local gsub_limit_ok, gsub_limit_message = pcall(string.gsub, "abc", "a", "b", false)

return upper_ok,
  string.byte(type(upper_message), 1),
  reverse_ok,
  string.byte(type(reverse_message), 1),
  byte_end_ok,
  string.byte(type(byte_end_message), 1),
  sub_end_ok,
  string.byte(type(sub_end_message), 1),
  find_subject_ok,
  string.byte(type(find_subject_message), 1),
  find_init_ok,
  string.byte(type(find_init_message), 1),
  match_subject_ok,
  string.byte(type(match_subject_message), 1),
  match_pattern_ok,
  string.byte(type(match_pattern_message), 1),
  match_init_ok,
  string.byte(type(match_init_message), 1),
  gmatch_subject_ok,
  string.byte(type(gmatch_subject_message), 1),
  gmatch_pattern_ok,
  string.byte(type(gmatch_pattern_message), 1),
  gsub_subject_ok,
  string.byte(type(gsub_subject_message), 1),
  gsub_pattern_ok,
  string.byte(type(gsub_pattern_message), 1),
  gsub_replacement_ok,
  string.byte(type(gsub_replacement_message), 1),
  gsub_limit_ok,
  string.byte(type(gsub_limit_message), 1)
