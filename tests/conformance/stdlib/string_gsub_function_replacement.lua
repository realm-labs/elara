local replaced, count = string.gsub("abc123", "(%a+)(%d+)", string.upper)

return #replaced, string.byte(replaced, 1), string.byte(replaced, 3), count
