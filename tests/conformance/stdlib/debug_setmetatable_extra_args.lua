local t = {}
local mt = {}

local updated = debug.setmetatable(t, mt, "ignored", false)
local raw = debug.getmetatable(t)
local cleared = debug.setmetatable(t, nil, "ignored")

return rawequal(updated, t), rawequal(raw, mt), rawequal(cleared, t), rawequal(debug.getmetatable(t), nil)
