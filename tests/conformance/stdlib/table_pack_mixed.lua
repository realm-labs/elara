local packed = table.pack(7, "az", false)

return packed.n, packed[1], string.byte(packed[2], 2), packed[3]
