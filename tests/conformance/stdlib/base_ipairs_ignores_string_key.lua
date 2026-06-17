local values = {
  [1] = 10,
  [2] = 20,
  name = 99,
}

local count = 0
local sum = 0

for index, value in ipairs(values) do
  count = count + 1
  sum = sum + index + value
end

return count, sum
