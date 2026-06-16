local replaced, count = string.gsub("abc123", "(%a+)", {abc = "word"})

return #replaced, string.byte(replaced, 1), string.byte(replaced, 5),
  string.byte(replaced, 7), count
