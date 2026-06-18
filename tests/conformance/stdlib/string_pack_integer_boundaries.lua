local packed = string.pack("<bBi1I1", -128, 255, 127, 0)
local signed_byte, unsigned_byte, signed_one, unsigned_one, next_pos =
  string.unpack("<bBi1I1", packed)

return #packed,
  string.byte(packed, 1),
  string.byte(packed, 2),
  string.byte(packed, 3),
  string.byte(packed, 4),
  signed_byte,
  unsigned_byte,
  signed_one,
  unsigned_one,
  next_pos
