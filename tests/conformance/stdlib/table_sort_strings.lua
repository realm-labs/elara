local values = {"c", "a", "b"}
local _ = table.sort(values)

return string.byte(values[1], 1), string.byte(values[2], 1),
  string.byte(values[3], 1)
