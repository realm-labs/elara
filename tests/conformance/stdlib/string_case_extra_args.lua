local lower = string.lower("AbC", "ignored", nil)
local upper = string.upper("aBc", "ignored", nil)

return string.byte(lower, 1),
  string.byte(lower, 2),
  string.byte(lower, 3),
  string.byte(upper, 1),
  string.byte(upper, 2),
  string.byte(upper, 3)
