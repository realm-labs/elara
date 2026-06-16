local values = {}

for letter, digits in string.gmatch("a1 b22", "(%a)(%d+)") do
  values[#values + 1] = string.byte(letter, 1)
  values[#values + 1] = string.len(digits)
  values[#values + 1] = string.byte(digits, 1)
end

return table.unpack(values)
