local packed = table.pack(1, 2, nil)

return packed.n, packed[1], packed[2], rawequal(packed[3], nil)
