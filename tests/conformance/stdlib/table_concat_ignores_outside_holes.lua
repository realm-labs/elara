local values = { "a", "b", nil, "d" }
local joined = table.concat(values, "-", 1, 2)

return string.len(joined), string.byte(joined, 1),
  string.byte(joined, 2), string.byte(joined, 3),
  rawequal(values[3], nil), string.byte(values[4], 1)
