local function done()
  return 7
end

local co = coroutine.create(done)
local ok, value = coroutine.resume(co)
local dead_ok, dead_message = coroutine.resume(co)

return ok,
  value,
  dead_ok,
  string.byte(type(dead_message), 1),
  string.byte(coroutine.status(co), 1)
