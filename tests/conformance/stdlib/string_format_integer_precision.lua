local formatted = string.format("%.3d:%.3x", 7, 10)

return string.len(formatted), string.byte(formatted, 1),
  string.byte(formatted, 3), string.byte(formatted, 4),
  string.byte(formatted, 5), string.byte(formatted, 7)
