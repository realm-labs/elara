local find_start, find_end = string.find("", "")
local matched = string.match("", "")
local replaced, replacements = string.gsub("", "", "x")

local gmatch_count = 0
local gmatch_len = -1

for value in string.gmatch("", "") do
  gmatch_count = gmatch_count + 1
  gmatch_len = string.len(value)
end

return find_start, find_end, string.len(matched), string.len(replaced),
  string.byte(replaced, 1), replacements, gmatch_count, gmatch_len
