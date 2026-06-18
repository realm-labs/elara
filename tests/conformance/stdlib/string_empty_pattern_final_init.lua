local find_start, find_end = string.find("abc", "", 4)
local plain_start, plain_end = string.find("abc", "", 4, true)
local matched = string.match("abc", "", 4)
local past_find = rawequal(string.find("abc", "", 5), nil)
local past_match = rawequal(string.match("abc", "", 5), nil)

return find_start, find_end, plain_start, plain_end, string.len(matched),
  past_find, past_match
