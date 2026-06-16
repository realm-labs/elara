local sliced = string.sub("abcdef", 2, -2)

return string.len(sliced), string.byte(sliced, 1), string.byte(sliced, 4)
