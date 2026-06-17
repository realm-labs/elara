local first, second = string.byte("ABC", nil, nil)

return first, rawequal(second, nil)
