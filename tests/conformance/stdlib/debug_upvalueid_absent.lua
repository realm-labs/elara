local x = 1
local function read()
  return x
end

local missing_id = debug.upvalueid(read, 2)
local native_id = debug.upvalueid(print, 1)
local bad_target_ok = pcall(debug.upvalueid, 1, 1)
local bad_index_ok = pcall(debug.upvalueid, read, false)
local missing_join_ok = pcall(debug.upvaluejoin, read, 2, read, 1)
local native_target_ok = pcall(debug.upvaluejoin, print, 1, read, 1)
local native_source_ok = pcall(debug.upvaluejoin, read, 1, print, 1)

return missing_id == nil,
  native_id == nil,
  bad_target_ok,
  bad_index_ok,
  missing_join_ok,
  native_target_ok,
  native_source_ok
