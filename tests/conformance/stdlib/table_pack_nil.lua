local packed = table.pack(1, nil, 3)

return packed.n, packed[1], rawequal(packed[2], nil), packed[3]
