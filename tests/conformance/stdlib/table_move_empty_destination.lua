local source = { 1, 2 }
local destination = { 9 }

local moved = table.move(source, 3, 2, 2, destination)

return rawequal(moved, destination),
  destination[1],
  rawequal(destination[2], nil),
  source[1],
  source[2]
