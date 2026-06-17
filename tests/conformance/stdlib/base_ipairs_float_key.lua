local count = 0
local sum = 0

for index, value in ipairs({ [1.0] = 10 }) do
  count = count + 1
  sum = sum + index + value
end

return count, sum
