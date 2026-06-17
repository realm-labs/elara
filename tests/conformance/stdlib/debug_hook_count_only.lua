local function hook()
end

debug.sethook(hook, "", 3)
local current_hook, mask, count = debug.gethook()
local cleared = debug.sethook()

return current_hook == hook,
  string.len(mask),
  count,
  cleared == nil
