local data = string.char(9, 42, 77)
local positioned_value, next_pos = string.unpack("B", data, 2)

return #data, positioned_value, next_pos
