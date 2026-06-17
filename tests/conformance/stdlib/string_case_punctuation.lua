local lower = string.lower("A-9_Z!")
local upper = string.upper("a-9_z!")

return string.byte(lower, 1),
  string.byte(lower, 2),
  string.byte(lower, 4),
  string.byte(lower, 6),
  string.byte(upper, 1),
  string.byte(upper, 2),
  string.byte(upper, 4),
  string.byte(upper, 6)
