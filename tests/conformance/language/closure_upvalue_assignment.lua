local x = 40

local function increment()
  x = x + 1
  return x
end

local function read()
  return x
end

return increment(), read()
