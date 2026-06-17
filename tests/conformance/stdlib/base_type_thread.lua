local function worker()
end

local thread = coroutine.create(worker)

return string.byte(type(thread), 1)
