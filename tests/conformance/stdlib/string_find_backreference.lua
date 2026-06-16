local start_pos, end_pos, word = string.find("alo alo", "(%a+) %1")

return start_pos, end_pos, #word, string.byte(word, 1), string.byte(word, 3)
