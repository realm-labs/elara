local function pair()
  return 20, 30
end

local function count(...)
  return select("#", ...), select(2, ...)
end

local total, first, second = count(10, pair())

return total, first, second
