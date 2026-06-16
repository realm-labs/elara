local repeated = string.rep("ab", 3, ",")

return string.len(repeated), string.byte(repeated, 3), string.byte(repeated, 6)
