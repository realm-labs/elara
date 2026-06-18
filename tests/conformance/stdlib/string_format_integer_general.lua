local formatted = string.format("%d:%i:%d:%i:%u:%o:%x:%X:%d",
  7, 8.0, "12.0", "-2.0", 7, 8, 255, 255, "+0x10")

return string.len(formatted),
  string.byte(formatted, 1),
  string.byte(formatted, 3),
  string.byte(formatted, 5),
  string.byte(formatted, 8),
  string.byte(formatted, 11),
  string.byte(formatted, 13),
  string.byte(formatted, 16),
  string.byte(formatted, 19),
  string.byte(formatted, 23)
