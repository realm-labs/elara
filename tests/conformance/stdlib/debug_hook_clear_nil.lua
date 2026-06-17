local function hook()
end

debug.sethook(hook, "c", 1)
local before_hook = debug.gethook()
local cleared = debug.sethook(nil)
local after_hook = debug.gethook()

return before_hook == hook,
  cleared == nil,
  after_hook == nil
