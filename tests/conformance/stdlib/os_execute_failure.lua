local ok, label, code = os.execute("exit 7")

return rawequal(ok, nil), string.byte(type(label), 1), code
