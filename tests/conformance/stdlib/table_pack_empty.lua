local packed = table.pack()

return packed.n, rawequal(packed[1], nil)
