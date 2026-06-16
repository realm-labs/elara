local invalid = string.char(192, 128)
local codepoint_ok, codepoint_message = pcall(utf8.codepoint, invalid)

local multibyte = utf8.char(65, 233)
local offset_ok, offset_message = pcall(utf8.offset, multibyte, 1, 3)

return codepoint_ok,
  string.byte(type(codepoint_message), 1),
  offset_ok,
  string.byte(type(offset_message), 1)
