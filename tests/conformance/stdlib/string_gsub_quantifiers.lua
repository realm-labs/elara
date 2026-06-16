local replaced = string.gsub("aaabbb", "a+", "x")

return string.len(replaced), string.byte(replaced, 1), string.byte(replaced, 4)
