local x = 1
local y = 2

local function first()
  return x
end

local function second()
  return y
end

local getlocal_ok, getlocal_message = pcall(debug.getlocal, first, nil)
local getupvalue_ok, getupvalue_message = pcall(debug.getupvalue, first, nil)
local setupvalue_ok, setupvalue_message = pcall(debug.setupvalue, first, nil, 3)
local upvalueid_ok, upvalueid_message = pcall(debug.upvalueid, first, nil)
local upvaluejoin_ok, upvaluejoin_message = pcall(debug.upvaluejoin, first, nil, second, 1)
local sethook_ok, sethook_message = pcall(debug.sethook, first, nil)

return getlocal_ok,
  string.byte(type(getlocal_message), 1),
  getupvalue_ok,
  string.byte(type(getupvalue_message), 1),
  setupvalue_ok,
  string.byte(type(setupvalue_message), 1),
  upvalueid_ok,
  string.byte(type(upvalueid_message), 1),
  upvaluejoin_ok,
  string.byte(type(upvaluejoin_message), 1),
  sethook_ok,
  string.byte(type(sethook_message), 1)
