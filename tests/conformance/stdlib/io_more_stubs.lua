local input_result, input_message = io.input("__elara_missing_conformance_file__.lua")
local output_result, output_message = io.output("__elara_missing_conformance_file__.lua")
local lines_result, lines_message = io.lines("__elara_missing_conformance_file__.lua", "*l", 1)
local popen_result, popen_message = io.popen("echo elara", "r")
local read_result, read_message = io.read("*l", 1)

return rawequal(input_result, nil), string.byte(type(input_message), 1),
  rawequal(output_result, nil), string.byte(type(output_message), 1),
  rawequal(lines_result, nil), string.byte(type(lines_message), 1),
  rawequal(popen_result, nil), string.byte(type(popen_message), 1),
  rawequal(read_result, nil), string.byte(type(read_message), 1)
