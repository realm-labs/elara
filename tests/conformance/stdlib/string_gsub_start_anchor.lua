local replaced = string.gsub("abcabc", "^a.", "x")

return string.len(replaced), string.byte(replaced, 1), string.byte(replaced, 2)
