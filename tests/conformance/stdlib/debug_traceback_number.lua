local trace = debug.traceback(123)

return string.byte(trace, 1),
  string.byte(trace, 2),
  string.byte(trace, 3),
  string.byte(type(trace), 1)
