local values = { 1, 3, label = "kept" }

table.insert(values, 2, 2)

return values[1], values[2], values[3], string.len(values.label)
