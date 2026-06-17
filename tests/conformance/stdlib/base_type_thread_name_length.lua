local function worker()
end

local thread = coroutine.create(worker)

return string.len(type(thread))
