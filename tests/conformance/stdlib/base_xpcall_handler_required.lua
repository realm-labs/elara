local called_missing = false
local function worker_missing()
  called_missing = true
  return 41
end

local called_type = false
local function worker_type()
  called_type = true
  return 42
end

local missing_ok, missing_message = pcall(xpcall, worker_missing)
local type_ok, type_message = pcall(xpcall, worker_type, false)

return missing_ok,
  string.byte(type(missing_message), 1),
  called_missing,
  type_ok,
  string.byte(type(type_message), 1),
  called_type
