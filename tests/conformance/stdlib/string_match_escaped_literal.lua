local matched = string.match("a+b", "a%+")

return string.len(matched), string.byte(matched, 1), string.byte(matched, 2)
