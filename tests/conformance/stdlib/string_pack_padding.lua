local packed = string.pack("<BxB", 1, 2)
local first, second, next_pos = string.unpack("<BxB", packed)

return #packed,
  string.byte(packed, 1),
  string.byte(packed, 2),
  string.byte(packed, 3),
  first,
  second,
  next_pos
