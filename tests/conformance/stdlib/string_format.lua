local formatted = string.format("%04x:%+d", 255, 7)

return string.len(formatted), string.byte(formatted, 1),
  string.byte(formatted, 4), string.byte(formatted, 5),
  string.byte(formatted, 6)
