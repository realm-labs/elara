local trace = debug.traceback("boom", nil)

return string.byte(trace, 1)
