local packed = string.pack("<n", 3.5)
local number, next_pos = string.unpack("<n", packed)

return #packed,
  string.byte(packed, 1),
  string.byte(packed, 7),
  string.byte(packed, 8),
  number,
  next_pos
