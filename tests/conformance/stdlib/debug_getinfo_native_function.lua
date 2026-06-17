local info = debug.getinfo(print, "Suf")

return string.byte(info.what, 1),
  info.nups,
  info.nparams,
  info.isvararg,
  rawequal(info.func, print),
  info.currentline == nil
