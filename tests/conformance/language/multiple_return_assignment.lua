local function values()
  return 10, 20, 30
end

local a, b = values()
local c, d, e = 1, values()
local f, g = values(), 40

return a, b, c, d, e, f, g
