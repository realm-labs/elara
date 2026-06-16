local function before()
  return true
end

local values = table.pack(1, 2)
local _ = table.sort(values, before)

return values[1], values[2]
