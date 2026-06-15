local sum = 0
for i = 1, 5 do
  sum = sum + i
end

local x = 0
repeat
  x = x + 1
until true

while true do
  x = x + 1
  break
end

return sum + x
