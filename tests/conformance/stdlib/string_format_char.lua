local formatted = string.format("%c:%c", 65, 66)

return string.len(formatted), string.byte(formatted, 1),
  string.byte(formatted, 2), string.byte(formatted, 3)
