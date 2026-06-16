local abs_type_ok, abs_type_message = pcall(math.abs, false)
local log_value_ok, log_value_message = pcall(math.log, false)
local log_base_ok, log_base_message = pcall(math.log, 8, false)
local max_type_ok, max_type_message = pcall(math.max, 1, false)
local min_missing_ok, min_missing_message = pcall(math.min)

return abs_type_ok,
  string.byte(type(abs_type_message), 1),
  log_value_ok,
  string.byte(type(log_value_message), 1),
  log_base_ok,
  string.byte(type(log_base_message), 1),
  max_type_ok,
  string.byte(type(max_type_message), 1),
  min_missing_ok,
  string.byte(type(min_missing_message), 1)
