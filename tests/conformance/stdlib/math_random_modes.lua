local unit_type = math.type(math.random())
local full_integer_type = math.type(math.random(0))

return string.byte(unit_type, 1), string.byte(full_integer_type, 1),
  string.byte(full_integer_type, 7)
