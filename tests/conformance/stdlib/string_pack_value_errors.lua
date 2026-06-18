local signed_ok, signed_message = pcall(string.pack, "b", 128)
local unsigned_ok, unsigned_message = pcall(string.pack, "B", 256)
local char_ok, char_message = pcall(string.pack, "c1", "ab")
local zero_ok, zero_message = pcall(string.pack, "z", string.char(97, 0, 98))

return signed_ok,
  string.byte(type(signed_message), 1),
  unsigned_ok,
  string.byte(type(unsigned_message), 1),
  char_ok,
  string.byte(type(char_message), 1),
  zero_ok,
  string.byte(type(zero_message), 1)
