local sliced = string.sub("abcdef", 2, 4, "ignored")

return string.len(sliced), string.byte(sliced, 1), string.byte(sliced, 3)
