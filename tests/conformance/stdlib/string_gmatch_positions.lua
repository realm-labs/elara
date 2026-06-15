local total = 0

for start_pos, end_pos in string.gmatch("ab cd", "()%a+()") do
  total = total + start_pos + end_pos
end

return total
