local fmod_zero_ok, fmod_zero_message = pcall(math.fmod, 1, 0)
local random_range_ok, random_range_message = pcall(math.random, 2, 1)
local random_count_ok, random_count_message = pcall(math.random, 1, 2, 3)
local ult_type_ok, ult_type_message = pcall(math.ult, 1, false)

return fmod_zero_ok,
  string.byte(type(fmod_zero_message), 1),
  random_range_ok,
  string.byte(type(random_range_message), 1),
  random_count_ok,
  string.byte(type(random_count_message), 1),
  ult_type_ok,
  string.byte(type(ult_type_message), 1)
