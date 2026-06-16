local replaced = string.gsub("a(b(c)d)e", "%b()", "x")

return string.len(replaced), string.byte(replaced, 1), string.byte(replaced, 2), string.byte(replaced, 3)
