local values = {"a", "b"}
table.insert(values, 3, "c")

return string.byte(values[1], 1), string.byte(values[2], 1),
  string.byte(values[3], 1)
