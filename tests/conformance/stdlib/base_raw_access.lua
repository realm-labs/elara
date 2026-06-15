local t = {}
local written = rawset(t, "name", 42)

return rawget(t, "name"), rawequal(written, t), rawequal(rawget(t, "missing"), nil)
