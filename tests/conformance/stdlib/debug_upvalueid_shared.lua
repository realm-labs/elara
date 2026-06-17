local x = 41

local function first()
  return x
end

local function second()
  return x
end

return rawequal(debug.upvalueid(first, 1), debug.upvalueid(second, 1)),
  first(),
  second()
