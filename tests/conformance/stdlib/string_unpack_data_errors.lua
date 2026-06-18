local short_ok, short_message = pcall(string.unpack, "I4", string.char(1, 2))
local unfinished_ok, unfinished_message = pcall(string.unpack, "z", "abc")
local position_ok, position_message =
  pcall(string.unpack, "I4", string.char(0, 0, 0, 0), 6)

return short_ok,
  string.byte(type(short_message), 1),
  unfinished_ok,
  string.byte(type(unfinished_message), 1),
  position_ok,
  string.byte(type(position_message), 1)
