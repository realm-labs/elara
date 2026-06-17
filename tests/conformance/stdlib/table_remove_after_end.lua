local values = {1, 2}
local removed = table.remove(values, 3)

return rawequal(removed, nil), values[1], values[2], rawequal(values[3], nil)
