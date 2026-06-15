local values = {"a", "c"}
local _ = table.insert(values, 2, "b")

return string.byte(values[1], 1), string.byte(values[2], 1),
  string.byte(values[3], 1)
