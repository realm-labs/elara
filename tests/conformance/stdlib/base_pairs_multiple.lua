local count = 0
local sum = 0

for key, value in pairs({ 10, 20 }) do
  count = count + 1
  sum = sum + key + value
end

return count, sum
