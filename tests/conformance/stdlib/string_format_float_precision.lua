local formatted = string.format("%.2f:%.1e:%.1E:%.4g:%.4G:%.0g:%.0f",
  1.25, 12.5, 12.5, 12.5, 1200000.0, 12.5, 12.5)

return string.len(formatted),
  string.byte(formatted, 2),
  string.byte(formatted, 7),
  string.byte(formatted, 9),
  string.byte(formatted, 15),
  string.byte(formatted, 17),
  string.byte(formatted, 24),
  string.byte(formatted, 28),
  string.byte(formatted, 30),
  string.byte(formatted, 33),
  string.byte(formatted, 36),
  string.byte(formatted, 38),
  string.byte(formatted, 42)
