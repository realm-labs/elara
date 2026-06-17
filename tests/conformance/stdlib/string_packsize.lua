local fixed = string.packsize("bBhHjJTfdnxi1I2c3")
local aligned = string.packsize("!4bI4Xdb")
local unaligned = string.packsize("!1bI4Xdb")
local string_ok, string_message = pcall(string.packsize, "s")
local zero_ok, zero_message = pcall(string.packsize, "z")

return fixed, aligned, unaligned,
  string_ok, string.byte(type(string_message), 1),
  zero_ok, string.byte(type(zero_message), 1)
