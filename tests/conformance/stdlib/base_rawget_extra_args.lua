local values = { name = 42 }

return rawget(values, "name", "ignored")
