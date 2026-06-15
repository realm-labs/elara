local values = {10, 20, 30}
local indexed_sum = 0

for index, value in ipairs(values) do
  indexed_sum = indexed_sum + index + value
end

local pair_sum = 0
for key, value in pairs({41}) do
  pair_sum = key + value
end

return indexed_sum, pair_sum
