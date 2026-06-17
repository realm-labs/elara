local function eq()
  return true
end

local mt = {
  __eq = eq,
}

local left = setmetatable({}, mt)
local right = setmetatable({}, mt)

return left == right, rawequal(left, right), rawequal(left, left)
