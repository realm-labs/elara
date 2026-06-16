local bad_chunk_ok, bad_chunk_message = pcall(load, 1)
local bad_mode_ok, bad_mode_message = pcall(load, "return 1", nil, "B")
local file_loader, file_message = loadfile("__elara_missing_file_for_conformance__.lua")
local dofile_ok, dofile_message = pcall(dofile, "__elara_missing_file_for_conformance__.lua")

return bad_chunk_ok,
  string.byte(type(bad_chunk_message), 1),
  bad_mode_ok,
  string.byte(type(bad_mode_message), 1),
  file_loader == nil,
  string.byte(type(file_message), 1),
  dofile_ok,
  string.byte(type(dofile_message), 1)
