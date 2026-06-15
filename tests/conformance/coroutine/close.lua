local function worker()
  return 1
end

local co = coroutine.create(worker)
return coroutine.close(co)
