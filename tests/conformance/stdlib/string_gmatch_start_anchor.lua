local values = {}

for match in string.gmatch("a^b ^c", "^.") do
  values[#values + 1] = string.len(match)
  values[#values + 1] = string.byte(match, 1)
  values[#values + 1] = string.byte(match, 2)
end

return table.unpack(values)
