local word = string.match("alo alo", "(%a+) %1")

return #word, string.byte(word, 1), string.byte(word, 3)
