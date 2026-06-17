local function probe()
  local x = 42
  local missing = debug.getlocal(1, 2)
  return x, missing == nil
end

return probe()
