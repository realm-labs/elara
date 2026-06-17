local text = utf8.char(65, 66, 67)

return string.len(text),
  utf8.len(text),
  string.byte(text, 1),
  string.byte(text, 2),
  string.byte(text, 3)
