local bad_first_ok, bad_first_message = pcall(math.randomseed, false)
local bad_second_ok, bad_second_message = pcall(math.randomseed, 1, false)
local bad_count_ok, bad_count_message = pcall(math.randomseed, 1, 2, 3)

return bad_first_ok,
  string.byte(type(bad_first_message), 1),
  bad_second_ok,
  string.byte(type(bad_second_message), 1),
  bad_count_ok,
  string.byte(type(bad_count_message), 1)
