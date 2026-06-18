local source = {"a", "b"}
local destination = {}

local returned = table.move(source, 1, 2, -1, destination)

return rawequal(returned, destination),
  string.byte(destination[-1], 1), string.byte(destination[0], 1),
  string.byte(source[1], 1), string.byte(source[2], 1)
