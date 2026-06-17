local text = string.char(237, 160, 128)
local count = 0
local last_position = 0
local last_codepoint = 0

for position, codepoint in utf8.codes(text, true) do
  count = count + 1
  last_position = position
  last_codepoint = codepoint
end

return count, last_position, last_codepoint
