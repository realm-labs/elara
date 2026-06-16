local x = 1
local function read()
  return x
end

local missing_name = debug.getupvalue(read, 2)
local native_name = debug.getupvalue(print, 1)
local missing_set = debug.setupvalue(read, 2, 9)
local native_set = debug.setupvalue(print, 1, 9)
local bad_target_ok = pcall(debug.setupvalue, 1, 1, 1)
local bad_index_ok = pcall(debug.setupvalue, read, false, 1)
local missing_value_ok = pcall(debug.setupvalue, read, 1)

return missing_name == nil,
  native_name == nil,
  missing_set == nil,
  native_set == nil,
  bad_target_ok,
  bad_index_ok,
  missing_value_ok
