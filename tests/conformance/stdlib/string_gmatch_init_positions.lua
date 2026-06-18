local negative_count = 0
local negative_len = 0
local negative_first = 0
local negative_second = 0

for value in string.gmatch("abcabc", "b.", -3) do
  negative_count = negative_count + 1
  negative_len = string.len(value)
  negative_first = string.byte(value, 1)
  negative_second = string.byte(value, 2)
end

local zero_count = 0
local zero_last_first = 0

for value in string.gmatch("abcabc", "b.", 0) do
  zero_count = zero_count + 1
  zero_last_first = string.byte(value, 1)
end

local past_count = 0

for _ in string.gmatch("abcabc", "b.", 8) do
  past_count = past_count + 1
end

return negative_count, negative_len, negative_first, negative_second,
  zero_count, zero_last_first, past_count
