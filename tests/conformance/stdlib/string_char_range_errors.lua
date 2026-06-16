local negative_ok, negative_message = pcall(string.char, -1)
local upper_ok, upper_message = pcall(string.char, 256)
local later_arg_ok, later_arg_message = pcall(string.char, 65, 256)

return negative_ok,
  string.byte(type(negative_message), 1),
  upper_ok,
  string.byte(type(upper_message), 1),
  later_arg_ok,
  string.byte(type(later_arg_message), 1)
