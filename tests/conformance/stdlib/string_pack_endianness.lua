local packed = string.pack(">I2<I2", 4660, 4660)
local big, little, next_pos = string.unpack(">I2<I2", packed)

return #packed,
  string.byte(packed, 1),
  string.byte(packed, 2),
  string.byte(packed, 3),
  string.byte(packed, 4),
  big,
  little,
  next_pos
