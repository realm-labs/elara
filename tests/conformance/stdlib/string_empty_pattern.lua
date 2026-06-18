local first_start, first_end = string.find("abc", "")
local third_start, third_end = string.find("abc", "", 3)
local matched = string.match("abc", "")
local replaced, count = string.gsub("ab", "", "-")

return first_start, first_end, third_start, third_end, string.len(matched),
  string.len(replaced), string.byte(replaced, 1), string.byte(replaced, 2),
  string.byte(replaced, 3), string.byte(replaced, 4),
  string.byte(replaced, 5), count
