local matched = string.match("abcb", "a.-b")

return string.len(matched), string.byte(matched, 1), string.byte(matched, 2)
