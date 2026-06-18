local empty = {}
local empty_removed = table.remove(empty, 0)
local nonempty_ok, nonempty_error = pcall(table.remove, { 1 }, 0)

return rawequal(empty_removed, nil),
  rawequal(empty[0], nil),
  nonempty_ok,
  string.byte(type(nonempty_error), 1)
