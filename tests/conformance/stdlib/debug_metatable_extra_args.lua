local t = {}
local mt = {}

setmetatable(t, mt)
local raw = debug.getmetatable(t, "ignored", false)
local absent = debug.getmetatable(1, "ignored")

return rawequal(raw, mt), rawequal(absent, nil)
