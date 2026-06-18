local value = math.abs(math.mininteger)

return value == math.mininteger,
  string.byte(math.type(value), 1)
