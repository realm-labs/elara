local max_plus_one_ok, max_plus_one_message = pcall(utf8.char, 2147483648)
local later_arg_ok, later_arg_message = pcall(utf8.char, 65, 2147483648)

return max_plus_one_ok,
  string.byte(type(max_plus_one_message), 1),
  later_arg_ok,
  string.byte(type(later_arg_message), 1)
