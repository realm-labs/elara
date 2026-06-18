local packed = string.pack("B", 7, "ignored", false, nil)

return #packed, string.byte(packed, 1)
