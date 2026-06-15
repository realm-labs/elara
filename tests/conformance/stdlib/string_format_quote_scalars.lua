local formatted = string.format("%q:%q:%q:%q", nil, true, -7, 1.5)

return string.len(formatted), string.byte(formatted, 1),
  string.byte(formatted, 4), string.byte(formatted, 5),
  string.byte(formatted, 9), string.byte(formatted, 10),
  string.byte(formatted, 12), string.byte(formatted, 15)
