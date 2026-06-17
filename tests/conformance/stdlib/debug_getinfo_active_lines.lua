local function probe()
  return 42
end

local info = debug.getinfo(probe, "L")

local count = 0
local all_active = true
for line, active in pairs(info.activelines) do
  count = count + 1
  all_active = all_active and type(line) == "number" and active == true
end

return string.byte(type(info.activelines), 1),
  info.currentline == nil,
  info.what == nil,
  all_active
