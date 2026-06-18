local locale = os.setlocale(nil, nil)

return string.byte(locale, 1)
