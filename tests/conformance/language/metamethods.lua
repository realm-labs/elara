local meta = {}

local function add()
  return 42
end

meta.__add = add

local left = {}
local right = {}
local _ = setmetatable(left, meta)

return left + right
