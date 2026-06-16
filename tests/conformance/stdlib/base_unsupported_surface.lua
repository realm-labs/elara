local loader, load_message = load("return 1")
local file_loader, file_message = loadfile("chunk.lua")
local dofile_ok, dofile_message = pcall(dofile, "chunk.lua")
local gc_ok, gc_message = pcall(collectgarbage, "collect")
local warn_ok = pcall(warn, "first", "second")

return loader == nil,
  string.byte(type(load_message), 1),
  file_loader == nil,
  string.byte(type(file_message), 1),
  dofile_ok,
  string.byte(type(dofile_message), 1),
  gc_ok,
  string.byte(type(gc_message), 1),
  warn_ok
