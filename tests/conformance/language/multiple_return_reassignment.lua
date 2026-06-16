local function values()
  return 10, 20, 30
end

local a, b, c = 0, 0, 0
a, b = values()
c, a, b = 1, values()

return a, b, c
