local text = tostring("a" .. string.char(0) .. "b")

return string.len(text),
  string.byte(text, 1),
  string.byte(text, 2),
  string.byte(text, 3)
