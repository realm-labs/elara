local values = {4, 5, 6}
local moved = table.move(values, 1, 3, 1)

return rawequal(moved, values), values[1], values[2], values[3]
