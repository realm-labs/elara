local x = 1
local y = 2

local function first()
  return x
end

local function second()
  return y
end

return rawequal(debug.upvalueid(first, 1), debug.upvalueid(second, 1)),
  first(),
  second()
