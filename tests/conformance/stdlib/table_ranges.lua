local values = {"a", "b", "c", "d"}
local joined = table.concat(values, ":", 2, 3)

local destination = {}
local moved = table.move({10, 20, 30}, 2, 3, 1, destination)

return string.len(joined), string.byte(joined, 1), moved[1], moved[2],
  rawequal(moved, destination)
