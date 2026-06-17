local repeated = string.rep("ab", 1, ",")

return string.len(repeated), string.byte(repeated, 1), string.byte(repeated, 2)
