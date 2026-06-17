local info = debug.getinfo(1, "nr")

return info.ftransfer,
  info.ntransfer,
  info.what == nil,
  info.currentline == nil
