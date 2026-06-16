return rawequal(math.tointeger(7.5), nil),
  rawequal(math.tointeger("not-a-number"), nil),
  rawequal(math.tointeger(false), nil)
