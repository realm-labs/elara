local repeated = string.rep("ab", 3, ",")
local middle = string.sub("abcdef", 2, -2)
local reversed = string.reverse("abc")
local upper = string.upper("az")
local lower = string.lower("AZ")

return string.len(repeated), string.len(middle), string.byte(reversed, 1),
  string.byte(upper, 2), string.byte(lower, 1)
