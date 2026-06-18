local formatted = string.format("%q", string.char(97, 34, 92, 10))

return string.len(formatted),
  string.byte(formatted, 1),
  string.byte(formatted, 2),
  string.byte(formatted, 3),
  string.byte(formatted, 4),
  string.byte(formatted, 5),
  string.byte(formatted, 6),
  string.byte(formatted, 7),
  string.byte(formatted, 8),
  string.byte(formatted, 9)
