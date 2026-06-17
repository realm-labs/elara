local values = {}

for digits in string.gmatch("a1 b22 c", "%d+") do
  values[#values + 1] = string.len(digits)
  values[#values + 1] = string.byte(digits, 1)
end

return table.unpack(values)
