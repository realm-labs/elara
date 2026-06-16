local replaced = string.gsub("alo alo", "()[al]", "%1")

return string.len(replaced), string.byte(replaced, 1), string.byte(replaced, 2), string.byte(replaced, 5), string.byte(replaced, 6)
