local min_integer = math.min(3.5, 2, 2.0)
local max_integer = math.max(1.5, 7, 7.0)
local max_float = math.max(1.5, 7.0, 7)

return string.byte(math.type(min_integer), 1),
  string.byte(math.type(max_integer), 1),
  string.byte(math.type(max_float), 1),
  min_integer,
  max_integer,
  max_float
