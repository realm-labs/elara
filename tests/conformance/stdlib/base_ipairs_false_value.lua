local count = 0
local first_is_false = false
local sum = 0

for index, value in ipairs({ false, 20 }) do
  count = count + 1

  if index == 1 then
    first_is_false = rawequal(value, false)
  else
    sum = sum + index + value
  end
end

return count, first_is_false, sum
