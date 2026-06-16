local floor_ok, floor_message = pcall(math.floor, false)
local ceil_ok, ceil_message = pcall(math.ceil, false)
local sqrt_ok, sqrt_message = pcall(math.sqrt, false)
local sin_ok, sin_message = pcall(math.sin, false)
local exp_ok, exp_message = pcall(math.exp, false)
local ldexp_ok, ldexp_message = pcall(math.ldexp, 1, false)

return floor_ok,
  string.byte(type(floor_message), 1),
  ceil_ok,
  string.byte(type(ceil_message), 1),
  sqrt_ok,
  string.byte(type(sqrt_message), 1),
  sin_ok,
  string.byte(type(sin_message), 1),
  exp_ok,
  string.byte(type(exp_message), 1),
  ldexp_ok,
  string.byte(type(ldexp_message), 1)
