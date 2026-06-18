local formatted = string.format("%14a:%+14a:%014a:%-14a:% 14a:%#a:%#A:%#a",
  12.5, 12.5, 12.5, 12.5, 12.5, 8.0, 8.0, 0.0)

return string.len(formatted),
  string.byte(formatted, 1),
  string.byte(formatted, 7),
  string.byte(formatted, 21),
  string.byte(formatted, 31),
  string.byte(formatted, 39),
  string.byte(formatted, 54),
  string.byte(formatted, 60),
  string.byte(formatted, 67),
  string.byte(formatted, 80),
  string.byte(formatted, 85),
  string.byte(formatted, 88),
  string.byte(formatted, 95)
