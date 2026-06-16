local ok, label, code = os.execute("exit 0")

return ok, string.byte(type(label), 1), code
