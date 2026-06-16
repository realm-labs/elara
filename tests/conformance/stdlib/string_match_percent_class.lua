local matched = string.match("abc123", "%d%d")

return string.len(matched), string.byte(matched, 1), string.byte(matched, 2)
