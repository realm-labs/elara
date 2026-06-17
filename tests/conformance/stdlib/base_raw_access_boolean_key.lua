local values = {}
local written = rawset(values, true, 42)

return rawget(values, true), rawequal(written, values), rawequal(rawget(values, false), nil)
