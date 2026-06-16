local iterator = string.gmatch("a1 b22 c333", "%d+")

local first = iterator()
local second = iterator()
local third = iterator()

return
  string.len(first), string.byte(first, 1),
  string.len(second), string.byte(second, 1),
  string.len(third), string.byte(third, 1)
