local floor_value = math.floor(3.8)
local ceil_value = math.ceil(3.2)

return floor_value, ceil_value, string.byte(math.type(floor_value), 1),
  string.byte(math.type(ceil_value), 1)
