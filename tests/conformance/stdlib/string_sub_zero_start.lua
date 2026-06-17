local sliced = string.sub("abcdef", 0, 2)

return string.len(sliced), string.byte(sliced, 1), string.byte(sliced, 2)
