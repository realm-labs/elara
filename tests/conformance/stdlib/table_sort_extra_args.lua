local values = {3, 1, 2}

table.sort(values, nil, "ignored")

return values[1], values[2], values[3]
