local formatted = string.format("%+d:% d:%+05d", 7, 7, 7)

return string.len(formatted), string.byte(formatted, 1),
  string.byte(formatted, 3), string.byte(formatted, 4),
  string.byte(formatted, 7), string.byte(formatted, 8),
  string.byte(formatted, 11)
