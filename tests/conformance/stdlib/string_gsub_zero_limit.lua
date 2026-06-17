local replaced, count = string.gsub("a1b2", "%d", "x", 0)

return string.len(replaced), string.byte(replaced, 2), string.byte(replaced, 4), count
