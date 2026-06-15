local values = {name = 42}
local cleared = rawset(values, "name", nil)

return rawequal(cleared, values), rawequal(rawget(values, "name"), nil)
