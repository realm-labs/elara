local subject = "a^b $a a$b"
local caret_start, caret_end = string.find(subject, "a^b")
local dollar_start, dollar_end = string.find(subject, "$a")
local middle_dollar_start, middle_dollar_end = string.find(subject, "a$b")
local terminal_anchor_start = string.find("a$b", "b$")

return caret_start, caret_end, dollar_start, dollar_end,
  middle_dollar_start, middle_dollar_end, terminal_anchor_start
