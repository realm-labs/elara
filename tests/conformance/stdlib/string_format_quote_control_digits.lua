local formatted = string.format("%q", string.char(1) .. "2")

return string.len(formatted),
  string.byte(formatted, 1),
  string.byte(formatted, 2),
  string.byte(formatted, 3),
  string.byte(formatted, 4),
  string.byte(formatted, 5),
  string.byte(formatted, 6),
  string.byte(formatted, 7)
