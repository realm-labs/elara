local values = { 3, 1, 2, label = "kept" }

table.sort(values)

return values[1], values[2], values[3], string.len(values.label)
