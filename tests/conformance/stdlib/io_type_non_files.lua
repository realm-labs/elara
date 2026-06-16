return rawequal(io.type(false), nil),
  rawequal(io.type({}), nil),
  rawequal(io.type(print), nil),
  rawequal(io.type("not a file"), nil)
