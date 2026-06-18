local table_text, table_count = string.gsub("ab", "", {})

local function return_false()
  return false
end

local false_text, false_count = string.gsub("ab", "", return_false)

local function return_nil()
  return nil
end

local nil_text, nil_count = string.gsub("ab", "", return_nil)

return string.len(table_text), string.byte(table_text, 1),
  string.byte(table_text, 2), table_count,
  string.len(false_text), string.byte(false_text, 1),
  string.byte(false_text, 2), false_count,
  string.len(nil_text), string.byte(nil_text, 1),
  string.byte(nil_text, 2), nil_count
