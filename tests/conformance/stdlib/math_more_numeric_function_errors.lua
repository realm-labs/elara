local acos_ok, acos_message = pcall(math.acos, false)
local asin_ok, asin_message = pcall(math.asin, false)
local atan_y_ok, atan_y_message = pcall(math.atan, false)
local atan_x_ok, atan_x_message = pcall(math.atan, 1, false)
local cos_ok, cos_message = pcall(math.cos, false)
local tan_ok, tan_message = pcall(math.tan, false)
local deg_ok, deg_message = pcall(math.deg, false)
local rad_ok, rad_message = pcall(math.rad, false)
local frexp_ok, frexp_message = pcall(math.frexp, false)
local modf_ok, modf_message = pcall(math.modf, false)

return acos_ok,
  string.byte(type(acos_message), 1),
  asin_ok,
  string.byte(type(asin_message), 1),
  atan_y_ok,
  string.byte(type(atan_y_message), 1),
  atan_x_ok,
  string.byte(type(atan_x_message), 1),
  cos_ok,
  string.byte(type(cos_message), 1),
  tan_ok,
  string.byte(type(tan_message), 1),
  deg_ok,
  string.byte(type(deg_message), 1),
  rad_ok,
  string.byte(type(rad_message), 1),
  frexp_ok,
  string.byte(type(frexp_message), 1),
  modf_ok,
  string.byte(type(modf_message), 1)
