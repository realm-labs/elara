return rawequal(io.type(nil, "ignored"), nil),
  rawequal(io.type(1, false), nil),
  rawequal(io.type("not a file", "ignored"), nil)
