local string_ok, string_message = pcall(error, "string-level", "0")
local float_ok, float_message = pcall(error, "float-level", 0.9)
local bad_ok, bad_message = pcall(error, "bad-level", "bad")

return string_ok,
  string_message == "string-level",
  float_ok,
  float_message == "float-level",
  bad_ok,
  string.byte(type(bad_message), 1)
