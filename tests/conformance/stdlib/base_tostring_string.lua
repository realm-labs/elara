local text = tostring("abc")

return string.len(text), string.byte(text, 1),
  string.byte(text, 2), string.byte(text, 3)
