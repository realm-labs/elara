local values = {}

for start_pos, end_pos in string.gmatch("ab cd", "()%a+()") do
  values[#values + 1] = start_pos
  values[#values + 1] = end_pos
end

return table.unpack(values)
