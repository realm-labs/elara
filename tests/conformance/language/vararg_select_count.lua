local function count(...)
  return select("#", ...)
end

local function values(...)
  return select(2, ...)
end

local a, b = values(10, nil, 30)

return count(10, nil, 30), a == nil, b
