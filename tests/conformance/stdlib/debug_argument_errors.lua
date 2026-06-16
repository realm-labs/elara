local function target()
  local value = 1
  return value
end

local getinfo_missing_ok, getinfo_missing_message = pcall(debug.getinfo)
local getinfo_target_ok, getinfo_target_message = pcall(debug.getinfo, false)
local getinfo_options_ok, getinfo_options_message = pcall(debug.getinfo, 1, false)
local getlocal_target_ok, getlocal_target_message = pcall(debug.getlocal, false, 1)
local getlocal_index_ok, getlocal_index_message = pcall(debug.getlocal, target, false)
local setlocal_level_ok, setlocal_level_message = pcall(debug.setlocal, false, 1, 1)
local getupvalue_target_ok, getupvalue_target_message = pcall(debug.getupvalue, false, 1)
local getupvalue_index_ok, getupvalue_index_message = pcall(debug.getupvalue, target, false)
local setupvalue_target_ok, setupvalue_target_message = pcall(debug.setupvalue, false, 1, 1)
local upvalueid_index_ok, upvalueid_index_message = pcall(debug.upvalueid, target, false)
local sethook_mask_ok, sethook_mask_message = pcall(debug.sethook, target, false)
local traceback_level_ok, traceback_level_message = pcall(debug.traceback, nil, false)

return getinfo_missing_ok,
  string.byte(type(getinfo_missing_message), 1),
  getinfo_target_ok,
  string.byte(type(getinfo_target_message), 1),
  getinfo_options_ok,
  string.byte(type(getinfo_options_message), 1),
  getlocal_target_ok,
  string.byte(type(getlocal_target_message), 1),
  getlocal_index_ok,
  string.byte(type(getlocal_index_message), 1),
  setlocal_level_ok,
  string.byte(type(setlocal_level_message), 1),
  getupvalue_target_ok,
  string.byte(type(getupvalue_target_message), 1),
  getupvalue_index_ok,
  string.byte(type(getupvalue_index_message), 1),
  setupvalue_target_ok,
  string.byte(type(setupvalue_target_message), 1),
  upvalueid_index_ok,
  string.byte(type(upvalueid_index_message), 1),
  sethook_mask_ok,
  string.byte(type(sethook_mask_message), 1),
  traceback_level_ok,
  string.byte(type(traceback_level_message), 1)
