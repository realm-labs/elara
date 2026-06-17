local marker = { answer = 42 }
local values = {}

setmetatable(values, { __metatable = marker })

local protected = getmetatable(values)

return protected.answer, rawequal(protected, marker)
