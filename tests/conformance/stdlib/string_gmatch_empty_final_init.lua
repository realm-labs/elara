local final_count = 0
local final_len = -1

for value in string.gmatch("abc", "", 4) do
  final_count = final_count + 1
  final_len = string.len(value)
end

local past_count = 0

for _ in string.gmatch("abc", "", 5) do
  past_count = past_count + 1
end

return final_count, final_len, past_count
