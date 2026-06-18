local missing_char_size_ok, missing_char_size_message = pcall(string.pack, "c", "a")
local invalid_align_ok, invalid_align_message = pcall(string.pack, "!3i", 1)
local invalid_size_ok, invalid_size_message = pcall(string.pack, "i17", 1)
local invalid_option_ok, invalid_option_message = pcall(string.pack, "Q", 1)
local invalid_next_ok, invalid_next_message = pcall(string.pack, "Xc1")

return missing_char_size_ok,
  string.byte(type(missing_char_size_message), 1),
  invalid_align_ok,
  string.byte(type(invalid_align_message), 1),
  invalid_size_ok,
  string.byte(type(invalid_size_message), 1),
  invalid_option_ok,
  string.byte(type(invalid_option_message), 1),
  invalid_next_ok,
  string.byte(type(invalid_next_message), 1)
