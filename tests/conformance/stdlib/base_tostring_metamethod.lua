local function custom_tostring(value)
  return "custom", value
end

local function numeric_tostring()
  return 42
end

local function bad_tostring()
  return false
end

local function raise_tostring()
  error(99)
end

local custom = setmetatable({}, { __tostring = custom_tostring })
local numeric = setmetatable({}, { __tostring = numeric_tostring })
local bad = setmetatable({}, { __tostring = bad_tostring })
local raises = setmetatable({}, { __tostring = raise_tostring })

local custom_text = tostring(custom)
local numeric_text = tostring(numeric)
local bad_ok, bad_message = pcall(tostring, bad)
local raise_ok, raise_message = pcall(tostring, raises)

return string.byte(custom_text, 1),
  string.len(custom_text),
  tonumber(numeric_text),
  bad_ok,
  string.byte(type(bad_message), 1),
  raise_ok,
  raise_message
