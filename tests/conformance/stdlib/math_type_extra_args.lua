return math.type(3, "ignored") == "integer",
  math.type(3.5, "ignored") == "float",
  rawequal(math.type(false, "ignored"), nil)
