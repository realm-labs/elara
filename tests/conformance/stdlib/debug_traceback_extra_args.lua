local trace = debug.traceback("boom", 1, "ignored")

return string.byte(trace, 1)
