local replaced = string.gsub("a1b2", "%d", "x")

return string.len(replaced), string.byte(replaced, 2), string.byte(replaced, 4)
