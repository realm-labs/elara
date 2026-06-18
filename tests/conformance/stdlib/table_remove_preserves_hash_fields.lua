local values = { 1, 2, 3, label = "kept" }
local removed = table.remove(values, 2)

return removed, values[1], values[2], rawequal(values[3], nil),
  string.len(values.label)
