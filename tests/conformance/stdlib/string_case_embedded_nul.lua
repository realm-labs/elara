local lower = string.lower("A" .. string.char(0) .. "Z")
local upper = string.upper("a" .. string.char(0) .. "z")

return string.len(lower),
  string.byte(lower, 1),
  string.byte(lower, 2),
  string.byte(lower, 3),
  string.len(upper),
  string.byte(upper, 1),
  string.byte(upper, 2),
  string.byte(upper, 3)
