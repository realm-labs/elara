local input_ok, input_message = pcall(io.input, false)
local output_ok, output_message = pcall(io.output, false)
local lines_file_ok, lines_file_message = pcall(io.lines, false)
local lines_format_ok, lines_format_message = pcall(io.lines, "file", false)
local open_file_ok, open_file_message = pcall(io.open, false)
local open_mode_ok, open_mode_message = pcall(io.open, "file", false)
local popen_command_ok, popen_command_message = pcall(io.popen, false)
local popen_mode_ok, popen_mode_message = pcall(io.popen, "cmd", false)
local read_ok, read_message = pcall(io.read, false)
local write_ok, write_message = pcall(io.write, false)

return input_ok,
  string.byte(type(input_message), 1),
  output_ok,
  string.byte(type(output_message), 1),
  lines_file_ok,
  string.byte(type(lines_file_message), 1),
  lines_format_ok,
  string.byte(type(lines_format_message), 1),
  open_file_ok,
  string.byte(type(open_file_message), 1),
  open_mode_ok,
  string.byte(type(open_mode_message), 1),
  popen_command_ok,
  string.byte(type(popen_command_message), 1),
  popen_mode_ok,
  string.byte(type(popen_mode_message), 1),
  read_ok,
  string.byte(type(read_message), 1),
  write_ok,
  string.byte(type(write_message), 1)
