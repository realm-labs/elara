local function probe()
  local x = 1
  local missing = debug.setlocal(1, 99, 42)
  return x, missing == nil
end

local value, missing = probe()
local bad_level_ok = pcall(debug.setlocal, false, 1, 2)
local bad_local_ok = pcall(debug.setlocal, 1, false, 2)
local missing_value_ok = pcall(debug.setlocal, 1, 1)
local bad_frame_ok = pcall(debug.setlocal, 99, 1, 2)

return value,
  missing,
  bad_level_ok,
  bad_local_ok,
  missing_value_ok,
  bad_frame_ok
