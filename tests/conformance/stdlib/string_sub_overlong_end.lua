local sliced = string.sub("abc", 2, 99)

return string.len(sliced), string.byte(sliced, 1), string.byte(sliced, 2)
