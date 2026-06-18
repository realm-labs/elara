local values = { "a", "b", label = "ignored" }
local joined = table.concat(values, "-")

return string.len(joined), string.byte(joined, 1),
  string.byte(joined, 2), string.byte(joined, 3),
  string.len(values.label)
