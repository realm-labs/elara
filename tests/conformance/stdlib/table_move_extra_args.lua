local source = {1, 2, 3}
local destination = {}
local moved = table.move(source, 1, 2, 2, destination, "ignored")

return rawequal(moved, destination), destination[2], destination[3]
