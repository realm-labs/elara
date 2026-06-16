local first = string.rep("b", 50)
local second = string.rep("a", 50)
local values = table.pack(first, second)
local _ = table.sort(values)

return string.byte(values[1], 1), string.len(values[1]),
  string.byte(values[2], 1), string.len(values[2])
