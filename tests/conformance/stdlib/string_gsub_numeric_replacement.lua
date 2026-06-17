local integer_replaced, integer_count = string.gsub("a1b2", "%d", 7)
local float_replaced, float_count = string.gsub("a1b2", "%d", 1.5)
local integral_float_replaced, integral_float_count = string.gsub("a1b2", "%d", 1.0)

return
  #integer_replaced, string.byte(integer_replaced, 2), integer_count,
  #float_replaced, string.byte(float_replaced, 2), string.byte(float_replaced, 3),
  string.byte(float_replaced, 4), float_count,
  #integral_float_replaced, string.byte(integral_float_replaced, 2),
  string.byte(integral_float_replaced, 3), string.byte(integral_float_replaced, 4),
  integral_float_count
