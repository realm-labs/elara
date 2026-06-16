local function hook()
end

debug.sethook(hook, "cr", 7)
local current_hook, mask, count = debug.gethook()
local cleared = debug.sethook()

return current_hook == hook,
  mask == "cr",
  count,
  cleared == nil,
  debug.gethook() == nil
