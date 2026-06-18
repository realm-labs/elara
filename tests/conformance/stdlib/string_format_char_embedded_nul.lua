local formatted = string.format("%c%c", 65, 256)

return string.len(formatted),
  string.byte(formatted, 1),
  string.byte(formatted, 2)
