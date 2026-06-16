local matched = string.match("abcabc", "^b.", 2)

return string.len(matched), string.byte(matched, 1), string.byte(matched, 2)
