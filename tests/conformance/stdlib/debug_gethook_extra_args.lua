local hook = debug.gethook("ignored", false)

return rawequal(hook, nil)
