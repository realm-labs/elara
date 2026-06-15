local t = {}
local mt = { __metatable = "locked" }
local replacement = {}

local installed = setmetatable(t, mt)
local protected = rawequal(getmetatable(t), "locked")
local raw_before = rawequal(debug.getmetatable(t), mt)
local updated = debug.setmetatable(t, replacement)
local raw_after = rawequal(debug.getmetatable(t), replacement)

return rawequal(installed, t), protected, raw_before, rawequal(updated, t), raw_after
