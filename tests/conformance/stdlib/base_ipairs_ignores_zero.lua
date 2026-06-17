local values = {
  [0] = 99,
  [1] = 10,
}

local count = 0
local sum = 0

for index, value in ipairs(values) do
  count = count + 1
  sum = sum + index + value
end

return count, sum
