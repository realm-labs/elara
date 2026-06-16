local matched = string.match("abcabc", "ca")

return string.len(matched), string.byte(matched, 1), string.byte(matched, 2)
