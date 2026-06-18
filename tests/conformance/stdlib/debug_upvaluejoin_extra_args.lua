local x = 1
local y = 2

local function first()
  return x
end

local function second()
  return y
end

local result = debug.upvaluejoin(first, 1, second, 1, "ignored", false)

return result == nil,
  first(),
  second(),
  rawequal(debug.upvalueid(first, 1), debug.upvalueid(second, 1))
