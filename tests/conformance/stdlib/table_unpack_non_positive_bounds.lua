local values = {
  [-1] = "a",
  [0] = "b",
  [1] = "c",
}
local first, second, third = table.unpack(values, -1, 1)

return string.byte(first, 1), string.byte(second, 1), string.byte(third, 1)
