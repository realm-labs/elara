local formatted = string.format("%f:%f:%f:%f:%e:%E:%g:%g:%G",
  7, 1.5, "2.25", "0x1.8p1", 12.5, 12.5, 12.5, 0.0000125, 1200000.0)

return string.len(formatted),
  string.byte(formatted, 1),
  string.byte(formatted, 2),
  string.byte(formatted, 19),
  string.byte(formatted, 28),
  string.byte(formatted, 45),
  string.byte(formatted, 58),
  string.byte(formatted, 63),
  string.byte(formatted, 65),
  string.byte(formatted, 72),
  string.byte(formatted, 73),
  string.byte(formatted, 80),
  string.byte(formatted, 83)
