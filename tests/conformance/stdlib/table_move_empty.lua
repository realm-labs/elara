local values = {1, 2}
local moved = table.move(values, 2, 1, 3)

return rawequal(moved, values), values[1], values[2], rawequal(values[3], nil)
