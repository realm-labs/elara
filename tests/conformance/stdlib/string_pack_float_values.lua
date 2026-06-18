local packed = string.pack("<fd", 1.5, -2.25)
local single, double, next_pos = string.unpack("<fd", packed)

return #packed,
  string.byte(packed, 1),
  string.byte(packed, 3),
  string.byte(packed, 4),
  string.byte(packed, 8),
  string.byte(packed, 9),
  string.byte(packed, 11),
  string.byte(packed, 12),
  single,
  double,
  next_pos
