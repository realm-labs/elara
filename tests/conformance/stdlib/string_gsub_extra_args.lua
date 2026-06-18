local replaced, count = string.gsub("a1b2c3", "%d", "x", 2, "ignored")

return string.byte(replaced, 2), string.byte(replaced, 4), string.byte(replaced, 6), count
