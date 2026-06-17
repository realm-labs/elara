local function done()
  return 5
end

local wrapped = coroutine.wrap(done)
local first = wrapped()
local ok, message = pcall(wrapped)

return first, ok, string.byte(type(message), 1)
