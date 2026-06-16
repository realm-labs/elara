local iterator = string.gmatch("a1 b22 c333", "%d+")

local first = iterator()
local second = iterator()
local third = iterator()

return string.len(first), string.len(second), string.len(third)
