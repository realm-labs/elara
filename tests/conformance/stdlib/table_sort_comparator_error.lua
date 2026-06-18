local function fail_compare()
  error("sort comparator failed")
end

local ok, message = pcall(table.sort, { 2, 1 }, fail_compare)

return ok, string.byte(type(message), 1)
