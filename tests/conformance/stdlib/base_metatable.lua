local t = {}
local mt = { __metatable = "locked" }
local installed = setmetatable(t, mt)
local protected = getmetatable(t)
local u = {}
local raw_mt = {}
local raw_installed = setmetatable(u, raw_mt)
local cleared = setmetatable(u, nil)

return rawequal(installed, t), rawequal(protected, "locked"),
  rawequal(raw_installed, u), rawequal(cleared, u), rawequal(getmetatable(u), nil),
  rawequal(getmetatable(1), nil)
