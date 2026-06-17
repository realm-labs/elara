local repeated = string.rep("ab", 2, nil)

return string.len(repeated), string.byte(repeated, 1), string.byte(repeated, 3),
  string.byte(repeated, 4)
