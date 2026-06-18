local values = {}
local mt = {}

local installed = setmetatable(values, mt, "ignored")
local observed = getmetatable(values, "ignored")

return rawequal(installed, values), rawequal(observed, mt)
