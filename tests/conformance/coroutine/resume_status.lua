local function answer()
  return 41, 42
end

local co = coroutine.create(answer)
local ok = coroutine.resume(co)
return ok
