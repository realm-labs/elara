local ascending = table.pack(3, 1, 2)
local _ = table.sort(ascending)

return ascending[1], ascending[2], ascending[3]
