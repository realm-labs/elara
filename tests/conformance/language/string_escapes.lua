local escaped = "a\n\t\\\"\'\x41\65\z
        b\u{20ac}"
local continued = "x\
y"
local b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12 = string.byte(escaped, 1, 12)
local c1, c2, c3 = string.byte(continued, 1, 3)

return #escaped, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, #continued, c1, c2, c3
