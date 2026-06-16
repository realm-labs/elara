local source = {1, 2, 3}
local destination = {}
local moved = table.move(source, 2, 3, 1, destination)

return rawequal(moved, destination), destination[1], destination[2], source[1], source[2], source[3]
