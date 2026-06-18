local values = { 1, 3, 2 }

local function descending(left, right)
  if left > right then
    return "yes"
  end
  return nil
end

table.sort(values, descending)

return values[1], values[2], values[3]
