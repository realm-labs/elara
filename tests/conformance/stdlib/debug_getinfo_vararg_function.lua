local function probe(a, ...)
  return a
end

local info = debug.getinfo(probe, "u")

return info.nparams, info.isvararg, info.what == nil
