local function probe()
  local x = 42
  local name = debug.setlocal(1, 1, 43)

  return x, string.byte(name, 1), string.byte(debug.getlocal(1, 1), 1),
    rawequal(debug.getlocal(probe, 1), nil)
end

return probe()
