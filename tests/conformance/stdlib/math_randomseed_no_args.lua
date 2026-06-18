local seed1, seed2 = math.randomseed()

return string.byte(math.type(seed1), 1), string.byte(math.type(seed2), 1)
