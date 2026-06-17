local slice = string.sub("ABC", 2, nil)

return string.len(slice), string.byte(slice, 1), string.byte(slice, 2)
