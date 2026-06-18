local min = math.min("b", "a", "c")
local max = math.max("b", "a", "c")

return string.byte(min, 1), string.byte(max, 1)
