local function probe()
  local info = debug.getinfo(1, "f")
  return rawequal(info.func, probe), info.what == nil
end

return probe()
