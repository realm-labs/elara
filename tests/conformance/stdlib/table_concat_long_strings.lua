local value = string.rep("a", 150)
local values = table.pack(value, value)
local joined = table.concat(values, "|")

return string.len(joined), string.byte(joined, 1), string.byte(joined, 151)
