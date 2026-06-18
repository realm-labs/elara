local values = {}
setmetatable(values, { __metatable = "locked" })

local ok, message = pcall(setmetatable, values, nil)

return ok, string.byte(type(message), 1)
