local before = debug.gethook()
local cleared = debug.sethook()
local after = debug.gethook()

return rawequal(before, nil), rawequal(cleared, nil), rawequal(after, nil)
