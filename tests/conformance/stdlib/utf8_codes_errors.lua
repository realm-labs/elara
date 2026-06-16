local bad_subject_ok, bad_subject_message = pcall(utf8.codes, false)
local bad_lead_ok, bad_lead_message = pcall(utf8.codes, string.char(128))

return bad_subject_ok,
  string.byte(type(bad_subject_message), 1),
  bad_lead_ok,
  string.byte(type(bad_lead_message), 1)
