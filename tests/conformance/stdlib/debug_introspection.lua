local function probe()
  return 1
end

local target = debug.getinfo(probe, "Su")
local here = debug.getinfo(1, "Sut")

return string.byte(target.what, 1), target.nparams, target.isvararg,
  string.byte(here.what, 1), here.nparams, here.isvararg, here.istailcall,
  here.extraargs, rawequal(debug.getinfo(1000), nil)
