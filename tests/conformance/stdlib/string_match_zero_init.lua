local matched = string.match("abc", "a", 0)

return string.len(matched), string.byte(matched, 1)
