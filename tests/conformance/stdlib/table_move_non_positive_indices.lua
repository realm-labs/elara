local values = {}
values[-1] = "a"
values[0] = "b"
values[1] = "c"

local returned = table.move(values, -1, 1, 2)

return rawequal(returned, values),
  string.byte(values[2], 1), string.byte(values[3], 1),
  string.byte(values[4], 1), string.byte(values[-1], 1),
  string.byte(values[0], 1), string.byte(values[1], 1)
