local abs_ok, abs_message = pcall(math.abs, nil)
local fmod_left_ok, fmod_left_message = pcall(math.fmod, nil, 1)
local fmod_right_ok, fmod_right_message = pcall(math.fmod, 1, nil)
local max_ok, max_message = pcall(math.max, nil)
local min_ok, min_message = pcall(math.min, nil)
local ult_ok, ult_message = pcall(math.ult, nil, 1)
local ldexp_exp_ok, ldexp_exp_message = pcall(math.ldexp, 1, nil)

return abs_ok,
  string.byte(type(abs_message), 1),
  fmod_left_ok,
  string.byte(type(fmod_left_message), 1),
  fmod_right_ok,
  string.byte(type(fmod_right_message), 1),
  max_ok,
  string.byte(type(max_message), 1),
  min_ok,
  string.byte(type(min_message), 1),
  ult_ok,
  string.byte(type(ult_message), 1),
  ldexp_exp_ok,
  string.byte(type(ldexp_exp_message), 1)
