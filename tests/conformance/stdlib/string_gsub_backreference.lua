local replaced = string.gsub("alo alo xyz", "(%a+) %1", "dup")

return string.len(replaced), string.byte(replaced, 1), string.byte(replaced, 5), string.byte(replaced, 7)
