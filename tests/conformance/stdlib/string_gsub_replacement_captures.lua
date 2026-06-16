local replaced = string.gsub("abc123", "(%a+)(%d+)", "%2-%1-%0-%%")

return string.len(replaced), string.byte(replaced, 1), string.byte(replaced, 5), string.byte(replaced, 16)
