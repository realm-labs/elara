local text = utf8.char(1114111)

return string.len(text), utf8.len(text), utf8.codepoint(text)
