local sliced = string.sub("abcdef", -3, -1)

return string.len(sliced), string.byte(sliced, 1), string.byte(sliced, 3)
