local formatted = string.format("%+8.2f:% 8.2f:%08.2f:%+08.2f:%-8.2f",
  1.25, 1.25, 1.25, 1.25, 1.25)

return string.len(formatted),
  string.byte(formatted, 1),
  string.byte(formatted, 4),
  string.byte(formatted, 6),
  string.byte(formatted, 10),
  string.byte(formatted, 14),
  string.byte(formatted, 19),
  string.byte(formatted, 23),
  string.byte(formatted, 28),
  string.byte(formatted, 29),
  string.byte(formatted, 33),
  string.byte(formatted, 37),
  string.byte(formatted, 41)
