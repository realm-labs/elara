local formatted = string.format("%#.0f:%#.0e:%#.4g:%#.4G:%#8.4g:%#.0g",
  12.5, 12.5, 12.5, 1200000.0, 12.5, 12.5)

return string.len(formatted),
  string.byte(formatted, 3),
  string.byte(formatted, 6),
  string.byte(formatted, 7),
  string.byte(formatted, 14),
  string.byte(formatted, 23),
  string.byte(formatted, 28),
  string.byte(formatted, 31),
  string.byte(formatted, 33),
  string.byte(formatted, 38),
  string.byte(formatted, 39),
  string.byte(formatted, 42)
