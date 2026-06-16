local values = {1, 2, 3, 4}
local moved = table.move(values, 2, 4, 1)

return rawequal(moved, values), values[1], values[2], values[3], values[4]
