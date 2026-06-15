local values = {1, 2, 3}
local _ = table.move(values, 1, 2, 2)

return values[1], values[2], values[3]
