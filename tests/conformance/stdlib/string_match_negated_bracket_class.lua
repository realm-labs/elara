local matched = string.match("abc123", "[^a-c][0-9]")

return string.len(matched), string.byte(matched, 1), string.byte(matched, 2)
