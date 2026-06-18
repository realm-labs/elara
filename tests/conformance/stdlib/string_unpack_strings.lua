local data = string.char(3, 97, 98, 99, 111, 107, 0)
local counted, zero, next_pos = string.unpack("<s1z", data)

return #counted,
  string.byte(counted, 1),
  string.byte(counted, 2),
  string.byte(counted, 3),
  #zero,
  string.byte(zero, 1),
  string.byte(zero, 2),
  next_pos
