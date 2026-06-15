local formatted = string.format("a%%b%%%%")

return string.len(formatted), string.byte(formatted, 1),
  string.byte(formatted, 2), string.byte(formatted, 3),
  string.byte(formatted, 4), string.byte(formatted, 5)
