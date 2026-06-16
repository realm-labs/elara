local function add(a, b)
  return a + b
end

local function missing(a, b)
  return a, b
end

local x, y = missing(7)

return add(20, 22), x, y
