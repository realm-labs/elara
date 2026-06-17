local values = { 10, 20, 30 }
local removed = table.remove(values, nil)

return removed, values[1], values[2], rawequal(values[3], nil)
