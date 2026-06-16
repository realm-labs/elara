local pointer = string.format("%p", "ab")

return string.byte(type(pointer), 1)
