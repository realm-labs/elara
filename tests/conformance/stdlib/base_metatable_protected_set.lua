local values = {}
local _ = setmetatable(values, { __metatable = "locked" })

return pcall(setmetatable, values, {})
