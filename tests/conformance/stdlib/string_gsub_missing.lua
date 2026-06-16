local replaced, count = string.gsub("abc", "%d", "x")

return #replaced, string.byte(replaced, 1), string.byte(replaced, 3), count
