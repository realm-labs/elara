local values = {}
local _ = table.sort(values)

return #values, rawequal(values[1], nil)
