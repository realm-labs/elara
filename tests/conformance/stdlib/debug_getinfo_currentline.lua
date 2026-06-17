local here = debug.getinfo(1, "l")

local function probe()
  local inside = debug.getinfo(1, "l")
  return type(inside.currentline), inside.what == nil, inside.func == nil
end

local inside_type, inside_no_what, inside_no_func = probe()

return string.byte(type(here.currentline), 1),
  here.what == nil,
  here.func == nil,
  string.byte(inside_type, 1),
  inside_no_what,
  inside_no_func
