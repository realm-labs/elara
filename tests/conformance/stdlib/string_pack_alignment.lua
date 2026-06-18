local packed = string.pack(">!4bI4Xdb", 1, 16909060, 0)
local aligned_byte, aligned_integer, aligned_tail, next_pos =
  string.unpack(">!4bI4Xdb", packed)

return #packed,
  string.byte(packed, 1),
  string.byte(packed, 2),
  string.byte(packed, 3),
  string.byte(packed, 4),
  string.byte(packed, 5),
  string.byte(packed, 6),
  string.byte(packed, 7),
  string.byte(packed, 8),
  string.byte(packed, 9),
  aligned_byte,
  aligned_integer,
  aligned_tail,
  next_pos
