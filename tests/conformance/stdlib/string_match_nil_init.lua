local matched = string.match("abcabc", "bc", nil)

return string.len(matched), string.byte(matched, 1), string.byte(matched, 2)
