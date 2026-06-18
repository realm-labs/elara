local function probe()
  return 1
end

local info = debug.getinfo(probe, "f", "ignored", false)

return rawequal(info.func, probe), info.what == nil, info.currentline == nil
