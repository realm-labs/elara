local formatted = string.format("%-4d:%04d", 7, 7)

return string.len(formatted), string.byte(formatted, 1),
  string.byte(formatted, 2), string.byte(formatted, 4),
  string.byte(formatted, 5), string.byte(formatted, 6),
  string.byte(formatted, 9)
