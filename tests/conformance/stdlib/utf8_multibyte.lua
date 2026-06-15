local text = utf8.char(233, 119070)
local second = utf8.offset(text, 2)

return string.len(text), utf8.len(text), utf8.codepoint(text, 3), second
