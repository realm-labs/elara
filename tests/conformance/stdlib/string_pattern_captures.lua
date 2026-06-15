local total = 0

for letter, digits in string.gmatch("a1 b22", "(%a)(%d+)") do
  total = total + string.len(letter) + string.len(digits)
end

local replaced = string.gsub("abc123", "(%a+)(%d+)", "%2-%1")

return total, string.len(replaced)
