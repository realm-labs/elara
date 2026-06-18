local near_end = string.match("abcabc", "b.", -3)
local clamped = string.match("abcabc", "b.", -99)

return string.len(near_end), string.byte(near_end, 1), string.byte(near_end, 2),
  string.len(clamped), string.byte(clamped, 1), string.byte(clamped, 2)
