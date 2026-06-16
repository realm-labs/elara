local matched = string.match("abc 123", "%f[%d]%d+")

return string.len(matched), string.byte(matched, 1), string.byte(matched, 3)
