local numeric = os.setlocale("C", "numeric", "ignored")

return string.byte(numeric, 1)
