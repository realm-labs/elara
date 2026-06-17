local repeated = string.rep(12, 2)
local slice = string.sub(12345, 2, 4)

return string.len(123), string.byte(65), string.len(1.0), string.byte(1.0, 2),
  string.len(repeated),
  string.byte(repeated, 1), string.byte(repeated, 4), string.len(slice),
  string.byte(slice, 1), string.byte(slice, 3)
