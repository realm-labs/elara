local replaced = string.gsub("abab", "a", "x")

return string.len(replaced), string.byte(replaced, 1), string.byte(replaced, 3)
