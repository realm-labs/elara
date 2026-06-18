local repeated = string.rep("ab", 2, ".", "ignored")

return string.len(repeated), string.byte(repeated, 3), string.byte(repeated, 5)
