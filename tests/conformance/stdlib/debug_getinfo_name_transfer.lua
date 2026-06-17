local info = debug.getinfo(1, "nr")

return string.len(info.namewhat),
  info.name == nil,
  info.ftransfer,
  info.ntransfer,
  info.what == nil,
  info.currentline == nil
