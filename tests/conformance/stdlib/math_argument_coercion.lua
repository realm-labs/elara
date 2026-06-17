local abs_value = math.abs("3")
local floor_value = math.floor("3.8")
local fmod_value = math.fmod("7", "2")
local sqrt_value = math.sqrt("9")
local ldexp_value = math.ldexp("0.75", "4")
local seed_first, seed_second = math.randomseed("123", "0x4")
local random_value = math.random("1", "1")
local ult_value = math.ult("1", "0x2")
local bad_random_ok = pcall(math.random, "1.5")
local bad_ldexp_ok = pcall(math.ldexp, 1, "1.5")

return string.byte(math.type(abs_value), 1),
  floor_value,
  string.byte(math.type(fmod_value), 1),
  sqrt_value,
  ldexp_value,
  seed_first,
  seed_second,
  random_value,
  ult_value,
  bad_random_ok,
  bad_ldexp_ok
