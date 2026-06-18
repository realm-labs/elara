local lower = string.lower(123)
local upper = string.upper(456)

return string.len(lower),
  string.byte(lower, 1),
  string.byte(lower, 3),
  string.len(upper),
  string.byte(upper, 1),
  string.byte(upper, 3)
