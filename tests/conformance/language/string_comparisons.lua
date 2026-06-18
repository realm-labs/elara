local left = "alpha"
local right = "beta"
local longer_left = "alphabet"
local longer_right = "alphazeta"

return left < right,
    left <= left,
    right > left,
    right >= right,
    not (right < left),
    "same" <= "same",
    longer_left < longer_right,
    "gamma" >= "beta"
