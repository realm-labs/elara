local function probe()
  local x = 1
  local missing = debug.setlocal(1, 2, 42)
  return x, missing == nil
end

return probe()
