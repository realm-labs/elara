local suffix = string.sub("abcd", 3)

return string.len(suffix), string.byte(suffix, 1), string.byte(suffix, 2)
