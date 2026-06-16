local matched = string.match("aaab", "ac?b")

return string.len(matched), string.byte(matched, 1), string.byte(matched, 2)
