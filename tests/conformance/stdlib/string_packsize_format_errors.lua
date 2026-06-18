local missing_char_size_ok, missing_char_size_message = pcall(string.packsize, "c")
local invalid_align_ok, invalid_align_message = pcall(string.packsize, "!3i")
local invalid_size_ok, invalid_size_message = pcall(string.packsize, "i17")
local invalid_option_ok, invalid_option_message = pcall(string.packsize, "Q")
local invalid_next_ok, invalid_next_message = pcall(string.packsize, "Xc1")

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
