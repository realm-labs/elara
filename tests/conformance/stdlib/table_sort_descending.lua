local values = { 1, 3, 2 }

local function descending(left, right)
  return left > right
end

table.sort(values, descending)

return values[1], values[2], values[3]
