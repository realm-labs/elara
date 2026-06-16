local matched = string.match("a(b(c)d)e", "%b()")

return string.len(matched), string.byte(matched, 1), string.byte(matched, 7)
