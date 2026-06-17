local values = { 1, 3 }

table.insert(values, 2, nil)

return values[1], values[2] == nil, values[3]
