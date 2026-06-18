local decimal_precision_ok, decimal_precision_message = pcall(string.format, "%.123f", 1.0)
local hex_precision_ok, hex_precision_message = pcall(string.format, "%.123a", 1.0)

return decimal_precision_ok,
  string.byte(type(decimal_precision_message), 1),
  hex_precision_ok,
  string.byte(type(hex_precision_message), 1)
