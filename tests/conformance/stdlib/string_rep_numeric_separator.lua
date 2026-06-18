local repeated = string.rep("x", 3, 9)

return string.len(repeated), string.byte(repeated, 2), string.byte(repeated, 5)
