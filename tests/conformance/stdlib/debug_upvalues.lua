local x = 10
local y = 20

local function left()
  return x
end

local function right()
  return y
end

local same_before = rawequal(debug.upvalueid(left, 1), debug.upvalueid(right, 1))
local old = left()
local name = debug.getupvalue(left, 1)
local set_name = debug.setupvalue(left, 1, 30)
local after_set = left()

local joined = debug.upvaluejoin(right, 1, left, 1)
local same_after = rawequal(debug.upvalueid(left, 1), debug.upvalueid(right, 1))
local updated = debug.setupvalue(right, 1, 40)

return old, after_set, left(), right(), same_before, same_after, rawequal(name, set_name)
