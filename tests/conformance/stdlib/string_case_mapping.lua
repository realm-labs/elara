local lower = string.lower("AbC123")
local upper = string.upper("AbC123")

return string.byte(lower, 1), string.byte(lower, 2), string.byte(lower, 3),
  string.byte(upper, 1), string.byte(upper, 2), string.byte(upper, 3)
