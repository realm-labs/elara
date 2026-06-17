local packed = table.pack("az", "by")

return packed.n, string.byte(packed[1], 1), string.byte(packed[2], 2)
