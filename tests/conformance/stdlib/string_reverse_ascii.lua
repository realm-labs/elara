local reversed = string.reverse("abcd")

return string.len(reversed), string.byte(reversed, 1), string.byte(reversed, 4)
