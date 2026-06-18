local source = { 1, 2 }
local destination = { label = "kept" }
local moved = table.move(source, 1, 2, 1, destination)

return rawequal(moved, destination), destination[1], destination[2],
  string.len(destination.label)
