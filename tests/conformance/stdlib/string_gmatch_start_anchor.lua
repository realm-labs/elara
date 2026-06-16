local total = 0

for match in string.gmatch("a^b ^c", "^.") do
  total = total + string.byte(match)
end

return total
