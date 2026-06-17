local values = { 10, 20 }

local written = rawset(values, 3, 30)
local cleared = rawset(values, 2, nil)

return rawget(values, 1),
  rawget(values, 3),
  rawget(values, 2) == nil,
  rawequal(written, values),
  rawequal(cleared, values)
