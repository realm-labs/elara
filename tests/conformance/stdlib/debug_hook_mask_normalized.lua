local function hook()
end

debug.sethook(hook, "lrcx", 5)
local current_hook, mask, count = debug.gethook()
local cleared = debug.sethook()

return current_hook == hook,
  string.len(mask),
  string.byte(mask, 1),
  string.byte(mask, 2),
  string.byte(mask, 3),
  count,
  cleared == nil
