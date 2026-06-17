local values = { 10, 20, 30 }

local function next_value(state, control)
  if control == nil then
    return 1, state[1]
  end

  if control < 3 then
    local next_index = control + 1
    return next_index, state[next_index]
  end
end

local sum = 0
local last_key = 0

for key, value in next_value, values, nil do
  sum = sum + key + value
  last_key = key
end

return sum, last_key
