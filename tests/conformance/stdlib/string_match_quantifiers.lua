local matched = string.match("aaab", "a+b")

return string.len(matched), string.byte(matched, 1), string.byte(matched, 4)
