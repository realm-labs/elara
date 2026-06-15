local formatted = string.format("%.2f:%+.1f", 1.5, 2)

return string.len(formatted), string.byte(formatted, 2),
  string.byte(formatted, 5), string.byte(formatted, 6)
