local t = {}
local mt = {}

debug.setmetatable(t, mt)
local before = debug.getmetatable(t)
local cleared = debug.setmetatable(t, nil)
local after = debug.getmetatable(t)

return rawequal(before, mt),
  rawequal(cleared, t),
  after == nil,
  getmetatable(t) == nil
