local values = {
  [-1] = "a",
  [0] = "b",
  [1] = "c",
}

local joined = table.concat(values, "", -1, 1)

return string.len(joined), string.byte(joined, 1), string.byte(joined, 2),
  string.byte(joined, 3)
