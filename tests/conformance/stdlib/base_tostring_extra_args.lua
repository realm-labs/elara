local text = tostring(42, "ignored")

return string.len(text), string.byte(text, 1), string.byte(text, 2)
