local value = string.char(65, 66, 67)

return string.len(value), string.byte(value, 1), string.byte(value, 3)
