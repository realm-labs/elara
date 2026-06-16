local values = {}
local _ = setmetatable(values, { __metatable = "locked" })
local ok, message = pcall(setmetatable, values, {})

return ok, string.byte(type(message), 1)
