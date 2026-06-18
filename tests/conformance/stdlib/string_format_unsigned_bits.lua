local formatted = string.format("%u:%x", -1, -1)

return string.len(formatted),
  string.byte(formatted, 1),
  string.byte(formatted, 2),
  string.byte(formatted, 10),
  string.byte(formatted, 20),
  string.byte(formatted, 21),
  string.byte(formatted, 22),
  string.byte(formatted, 37)
