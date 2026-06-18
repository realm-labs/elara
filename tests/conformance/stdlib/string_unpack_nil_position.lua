local value, next_pos = string.unpack("B", string.char(42, 99), nil, "ignored")

return value, next_pos
