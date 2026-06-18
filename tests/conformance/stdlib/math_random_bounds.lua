local upper = math.random(3)
local range = math.random(-2, 2)

return upper >= 1 and upper <= 3,
  string.byte(math.type(upper), 1),
  range >= -2 and range <= 2,
  string.byte(math.type(range), 1)
