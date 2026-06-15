local t = table.pack(3, 1, 2)
local _ = table.sort(t)

local gsub_len = string.len(string.gsub("a1b2", "%d", "x"))
local match_len = string.len(string.match("abc123", "%a+"))

local u = utf8.char(65, 66)
return t[1], t[2], t[3], gsub_len, match_len, utf8.len(u), utf8.codepoint(u, 2)
