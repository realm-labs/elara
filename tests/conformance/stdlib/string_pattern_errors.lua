local invalid_capture_ok, invalid_capture_message = pcall(string.find, "abc", "%0")
local trailing_escape_ok, trailing_escape_message = pcall(string.find, "abc", "%")
local missing_bracket_ok, missing_bracket_message = pcall(string.find, "abc", "[abc")
local missing_balanced_ok, missing_balanced_message = pcall(string.find, "abc", "%bx")
local missing_frontier_ok, missing_frontier_message = pcall(string.find, "abc", "%fa")
local empty_class_ok, empty_class_message = pcall(string.find, "abc", "[]")
local empty_frontier_ok, empty_frontier_message = pcall(string.find, "abc", "%f[]")
local unfinished_capture_ok, unfinished_capture_message = pcall(string.find, "abc", "(a")
local out_of_range_capture_ok, out_of_range_capture_message =
  pcall(string.find, "abc", "(%a+)%2")
local too_many_captures_ok, too_many_captures_message =
  pcall(string.find, "abc", string.rep("()", 33))

return invalid_capture_ok, string.byte(type(invalid_capture_message), 1),
  trailing_escape_ok, string.byte(type(trailing_escape_message), 1),
  missing_bracket_ok, string.byte(type(missing_bracket_message), 1),
  missing_balanced_ok, string.byte(type(missing_balanced_message), 1),
  missing_frontier_ok, string.byte(type(missing_frontier_message), 1),
  empty_class_ok, string.byte(type(empty_class_message), 1),
  empty_frontier_ok, string.byte(type(empty_frontier_message), 1),
  unfinished_capture_ok, string.byte(type(unfinished_capture_message), 1),
  out_of_range_capture_ok, string.byte(type(out_of_range_capture_message), 1),
  too_many_captures_ok, string.byte(type(too_many_captures_message), 1)
