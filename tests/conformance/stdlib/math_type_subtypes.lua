local integer_type = math.type(3)
local float_type = math.type(3.5)

return string.len(integer_type), string.byte(integer_type, 1),
  string.byte(integer_type, 7), string.len(float_type),
  string.byte(float_type, 1), string.byte(float_type, 5)
