local matched = string.match("abcabc", "bc", 2, "ignored")

return string.len(matched), string.byte(matched, 1), string.byte(matched, 2)
