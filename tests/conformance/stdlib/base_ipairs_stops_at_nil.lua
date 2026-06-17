local count = 0
local sum = 0

for index, value in ipairs({ 10, nil, 30 }) do
  count = count + 1
  sum = sum + index + value
end

return count, sum
