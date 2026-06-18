local random_ok, random_message = pcall(math.random, nil)
local randomseed_ok, randomseed_message = pcall(math.randomseed, nil)

return random_ok,
  string.byte(type(random_message), 1),
  randomseed_ok,
  string.byte(type(randomseed_message), 1)
