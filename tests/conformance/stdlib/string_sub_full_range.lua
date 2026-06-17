local sliced = string.sub("abc", 1, 3)

return string.len(sliced), string.byte(sliced, 1), string.byte(sliced, 3)
