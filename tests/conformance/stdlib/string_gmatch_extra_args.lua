local values = {}

for digits in string.gmatch("a1 b22 c333", "%d+", 3, "ignored") do
  values[#values + 1] = string.len(digits)
  values[#values + 1] = string.byte(digits, 1)
end

return table.unpack(values)
