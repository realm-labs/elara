local values = {}
local written = rawset(values, -2, 42)

return rawget(values, -2), rawequal(written, values), rawget(values, 2) == nil
