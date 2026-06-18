local packed = string.pack(">i2<i2", -2, -2)
local big, little, next_pos = string.unpack(">i2<i2", packed)

return #packed,
  string.byte(packed, 1),
  string.byte(packed, 2),
  string.byte(packed, 3),
  string.byte(packed, 4),
  big,
  little,
  next_pos
