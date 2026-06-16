local calls = 0

local function bump()
  calls = calls + 1
  return 99
end

local first = false and bump()
local second = true or bump()
local third = nil or 7
local fourth = 0 and 8
local fifth = nil and bump()

return first == false, second == true, third, fourth, fifth == nil, calls
