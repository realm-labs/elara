local function answer()
  return 1
end

local co = coroutine.create(answer)
local before = string.byte(coroutine.status(co), 1)
local yieldable = coroutine.isyieldable(co)
local ok = coroutine.resume(co)
local after = string.byte(coroutine.status(co), 1)

return before, yieldable, ok, after
