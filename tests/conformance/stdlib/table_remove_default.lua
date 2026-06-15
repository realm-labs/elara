local values = {1, 2}
local removed = table.remove(values)

return removed, values[1], rawequal(values[2], nil)
