local source = { 1, nil, 3 }
local destination = { 9, 9, 9 }

local moved = table.move(source, 1, 3, 1, destination)

return rawequal(moved, destination),
  destination[1],
  destination[2] == nil,
  destination[3]
