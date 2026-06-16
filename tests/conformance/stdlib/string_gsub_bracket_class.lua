local replaced = string.gsub("a1b2c3", "[%a][0-9]", "x")

return string.len(replaced), string.byte(replaced, 1), string.byte(replaced, 3)
