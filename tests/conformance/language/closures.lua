local x = 40

local function answer()
  return x + 2
end

local y = 41
local function other()
  return y + 1
end

return answer(), other()
