local wide_width_ok, wide_width_message = pcall(string.format, "%123x", 7)
local wide_precision_ok, wide_precision_message = pcall(string.format, "%.123d", 7)
local decimal_alternate_ok, decimal_alternate_message = pcall(string.format, "%#5d", 7)
local unsigned_sign_ok, unsigned_sign_message = pcall(string.format, "%+5u", 7)

return wide_width_ok,
  string.byte(type(wide_width_message), 1),
  wide_precision_ok,
  string.byte(type(wide_precision_message), 1),
  decimal_alternate_ok,
  string.byte(type(decimal_alternate_message), 1),
  unsigned_sign_ok,
  string.byte(type(unsigned_sign_message), 1)
