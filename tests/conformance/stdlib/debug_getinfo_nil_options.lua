local function probe()
  return 1
end

local info = debug.getinfo(probe, nil)

return string.byte(info.what, 1), info.nparams, info.isvararg,
  rawequal(info.func, probe)
