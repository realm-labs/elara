local matched = string.match("abcabc", "a.", 4)

return string.len(matched), string.byte(matched, 2)
