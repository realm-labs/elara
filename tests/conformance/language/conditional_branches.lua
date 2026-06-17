local first = 0
if false then
  first = 1
elseif true then
  first = 2
else
  first = 3
end

local second = 0
if nil then
  second = 4
else
  second = 5
end

local third = 0
if 0 then
  third = 6
end

return first, second, third
