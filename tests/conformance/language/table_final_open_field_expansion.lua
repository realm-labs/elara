local function tail()
  return 20, 30
end

local function none()
end

local function call_fields()
  local t = { 10, tail() }
  return rawlen(t), t[1], t[2], t[3]
end

local function vararg_fields(...)
  local t = { ... }
  return rawlen(t), t[1], t[3]
end

local function empty_tail()
  local t = { 10, none() }
  return rawlen(t), t[2] == nil
end

local call_len, call_first, call_second, call_third = call_fields()
local var_len, var_first, var_third = vararg_fields(40, 50, 60)
local empty_len, empty_second_missing = empty_tail()

return call_len, call_first, call_second, call_third,
  var_len, var_first, var_third,
  empty_len, empty_second_missing
