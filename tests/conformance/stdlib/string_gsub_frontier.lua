local replaced = string.gsub("abc 123 def 45", "%f[%d]%d+", "n")

return string.len(replaced), string.byte(replaced, 5), string.byte(replaced, 11)
