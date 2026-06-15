local formatted = string.format("%5.3s:%-4s", "abcdef", "xy")

return string.len(formatted), string.byte(formatted, 1),
  string.byte(formatted, 2), string.byte(formatted, 3),
  string.byte(formatted, 6), string.byte(formatted, 9),
  string.byte(formatted, 10)
