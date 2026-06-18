local string_base = tonumber("1010", "2")
local float_base = tonumber("1010", 2.9)
local bad_ok, bad_message = pcall(tonumber, "1010", "bad")

return string_base,
  float_base,
  bad_ok,
  string.byte(type(bad_message), 1)
