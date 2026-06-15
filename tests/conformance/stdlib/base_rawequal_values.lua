local left = {}
local right = {}

return rawequal(nil, nil), rawequal(1, 1.0), rawequal("a", "a"),
  rawequal(left, left), rawequal(left, right)
