local values = {}
setmetatable(values, { __metatable = false })

local observed = getmetatable(values)
local ok, message = pcall(setmetatable, values, {})

return observed == false, ok, string.byte(type(message), 1)
