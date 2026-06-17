local function worker()
end

local thread = coroutine.create(worker)
local text = tostring(thread)

return string.byte(text, 1), string.byte(text, 7)
