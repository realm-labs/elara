use std::{fs, path::PathBuf};

use elara_api::Lua;
use elara_core::Value;

#[test]
fn conformance_language_fixtures() {
    assert_success_fixture("language/return_42.lua", vec![Value::integer(42)]);
    assert_success_fixture("language/control_flow.lua", vec![Value::integer(17)]);
    assert_success_fixture(
        "language/bitwise.lua",
        vec![
            Value::integer(8),
            Value::integer(14),
            Value::integer(6),
            Value::integer(16),
            Value::integer(4),
            Value::integer(0),
            Value::integer(-1),
        ],
    );
    assert_success_fixture("language/varargs.lua", vec![Value::integer(10)]);
    assert_success_fixture(
        "language/table_fields.lua",
        vec![
            Value::integer(10),
            Value::integer(20),
            Value::integer(30),
            Value::integer(40),
            Value::integer(40),
            Value::integer(4),
        ],
    );
    assert_success_fixture(
        "language/closures.lua",
        vec![Value::integer(42), Value::integer(42)],
    );
    assert_success_fixture(
        "language/global_declarations.lua",
        vec![Value::integer(42), Value::integer(42)],
    );
    assert_success_fixture("language/metamethods.lua", vec![Value::integer(42)]);
}

#[test]
fn conformance_standard_library_fixtures() {
    assert_success_fixture("stdlib/math_abs.lua", vec![Value::integer(42)]);
    assert_success_fixture("stdlib/math_abs_float.lua", vec![Value::float(2.5)]);
    assert_success_fixture(
        "stdlib/base_table.lua",
        vec![
            Value::integer(110),
            Value::boolean(true),
            Value::boolean(true),
            Value::integer(3),
            Value::integer(2),
            Value::integer(100),
            Value::integer(101),
        ],
    );
    assert_success_fixture(
        "stdlib/base_rawlen.lua",
        vec![Value::integer(3), Value::integer(3)],
    );
    assert_success_fixture(
        "stdlib/base_rawlen_empty.lua",
        vec![Value::integer(0), Value::integer(0)],
    );
    assert_success_fixture(
        "stdlib/base_raw_access.lua",
        vec![Value::integer(42), Value::boolean(true), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/base_rawset_nil.lua",
        vec![Value::boolean(true), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/base_rawequal_values.lua",
        vec![
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(false),
        ],
    );
    assert_success_fixture(
        "stdlib/base_iteration.lua",
        vec![Value::integer(66), Value::integer(42)],
    );
    assert_success_fixture(
        "stdlib/base_metatable.lua",
        vec![
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture_values("stdlib/base_metatable_protected_set.lua", |actual| {
        assert_eq!(
            actual.len(),
            2,
            "protected metatable update should return status plus message"
        );
        assert_eq!(
            actual[0],
            Value::boolean(false),
            "protected metatable update should be caught"
        );
        assert!(
            actual[1].is_string(),
            "protected metatable message should be a string: {actual:?}"
        );
    });
    assert_success_fixture(
        "stdlib/base_assert.lua",
        vec![Value::boolean(true), Value::integer(42)],
    );
    assert_success_fixture_values("stdlib/base_assert_pcall.lua", |actual| {
        assert_eq!(
            actual.len(),
            2,
            "protected assert failure should return status plus message"
        );
        assert_eq!(
            actual[0],
            Value::boolean(false),
            "protected assert should catch false condition"
        );
        assert!(
            actual[1].is_string(),
            "protected assert message should be a string: {actual:?}"
        );
    });
    assert_success_fixture(
        "stdlib/base_next.lua",
        vec![Value::integer(1), Value::integer(10)],
    );
    assert_success_fixture("stdlib/base_next_empty.lua", vec![Value::boolean(true)]);
    assert_success_fixture(
        "stdlib/base_conversion.lua",
        vec![
            Value::integer(20),
            Value::integer(3),
            Value::integer(10),
            Value::boolean(true),
            Value::integer(116),
        ],
    );
    assert_success_fixture(
        "stdlib/base_tonumber_radix.lua",
        vec![
            Value::integer(255),
            Value::integer(35),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/base_tonumber_number.lua",
        vec![Value::integer(12), Value::float(12.5)],
    );
    assert_success_fixture(
        "stdlib/base_tonumber_standard.lua",
        vec![Value::integer(-12), Value::integer(16), Value::float(3.5)],
    );
    assert_success_fixture(
        "stdlib/base_type_values.lua",
        vec![
            Value::integer(110),
            Value::integer(98),
            Value::integer(110),
            Value::integer(115),
            Value::integer(116),
            Value::integer(102),
        ],
    );
    assert_success_fixture(
        "stdlib/base_tostring.lua",
        vec![
            Value::integer(110),
            Value::integer(3),
            Value::integer(102),
            Value::integer(5),
            Value::integer(45),
            Value::integer(3),
        ],
    );
    assert_success_fixture(
        "stdlib/base_select_multi.lua",
        vec![Value::integer(20), Value::integer(30)],
    );
    assert_success_fixture(
        "stdlib/base_select_count_empty.lua",
        vec![Value::integer(0)],
    );
    assert_success_fixture(
        "stdlib/base_pcall.lua",
        vec![Value::boolean(true), Value::integer(42)],
    );
    assert_success_fixture_values("stdlib/base_pcall_error.lua", |actual| {
        assert_eq!(actual.len(), 2, "pcall error should return status plus message");
        assert_eq!(actual[0], Value::boolean(false), "pcall should catch error");
        assert!(
            actual[1].is_string(),
            "pcall error message should be a string: {actual:?}"
        );
    });
    assert_success_fixture(
        "stdlib/base_xpcall.lua",
        vec![Value::boolean(false), Value::integer(9)],
    );
    assert_success_fixture(
        "stdlib/math_string_patterns.lua",
        vec![
            Value::integer(7),
            Value::integer(2),
            Value::integer(2),
            Value::integer(42),
            Value::integer(105),
            Value::integer(4),
            Value::integer(3),
            Value::integer(3),
        ],
    );
    assert_success_fixture(
        "stdlib/string_pattern_captures.lua",
        vec![Value::integer(5), Value::integer(7)],
    );
    assert_success_fixture(
        "stdlib/string_find_positions.lua",
        vec![Value::integer(4), Value::integer(5)],
    );
    assert_success_fixture(
        "stdlib/string_find_dot_wildcard.lua",
        vec![Value::integer(1), Value::integer(2)],
    );
    assert_success_fixture(
        "stdlib/string_find_bracket_class.lua",
        vec![Value::integer(4), Value::integer(5)],
    );
    assert_success_fixture(
        "stdlib/string_find_negated_bracket_class.lua",
        vec![Value::integer(4), Value::integer(5)],
    );
    assert_success_fixture(
        "stdlib/string_find_quantifiers.lua",
        vec![Value::integer(1), Value::integer(4)],
    );
    assert_success_fixture(
        "stdlib/string_find_optional_quantifier.lua",
        vec![Value::integer(3), Value::integer(4)],
    );
    assert_success_fixture(
        "stdlib/string_find_plain.lua",
        vec![Value::integer(4), Value::integer(4)],
    );
    assert_success_fixture(
        "stdlib/string_find_escaped_literal.lua",
        vec![Value::integer(1), Value::integer(2)],
    );
    assert_success_fixture(
        "stdlib/string_find_start_anchor_init.lua",
        vec![Value::integer(2), Value::integer(3)],
    );
    assert_success_fixture(
        "stdlib/string_find_end_anchor.lua",
        vec![Value::integer(5), Value::integer(6)],
    );
    assert_success_fixture_values("stdlib/string_find_captures.lua", |actual| {
        assert_eq!(
            actual.len(),
            4,
            "string.find capture fixture should return bounds and captures"
        );
        assert_eq!(actual[0], Value::integer(1));
        assert_eq!(actual[1], Value::integer(6));
        assert!(
            actual[2].is_string(),
            "string.find first capture should be a string: {actual:?}"
        );
        assert!(
            actual[3].is_string(),
            "string.find second capture should be a string: {actual:?}"
        );
    });
    assert_success_fixture(
        "stdlib/string_find_frontier.lua",
        vec![Value::integer(5), Value::integer(7)],
    );
    assert_success_fixture(
        "stdlib/string_find_balanced.lua",
        vec![Value::integer(2), Value::integer(8)],
    );
    assert_success_fixture_values("stdlib/string_find_backreference.lua", |actual| {
        assert_eq!(
            actual.len(),
            3,
            "string.find backreference fixture should return bounds and capture"
        );
        assert_eq!(actual[0], Value::integer(1));
        assert_eq!(actual[1], Value::integer(7));
        assert!(
            actual[2].is_string(),
            "string.find backreference capture should be a string: {actual:?}"
        );
    });
    assert_success_fixture(
        "stdlib/string_find_position_captures.lua",
        vec![
            Value::integer(3),
            Value::integer(4),
            Value::integer(3),
            Value::integer(5),
        ],
    );
    assert_success_fixture(
        "stdlib/string_find_missing.lua",
        vec![Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/string_position_captures.lua",
        vec![Value::integer(3), Value::integer(5)],
    );
    assert_success_fixture(
        "stdlib/string_match_literal.lua",
        vec![Value::integer(2), Value::integer(99), Value::integer(97)],
    );
    assert_success_fixture(
        "stdlib/string_match_dot_wildcard.lua",
        vec![Value::integer(2), Value::integer(98), Value::integer(99)],
    );
    assert_success_fixture(
        "stdlib/string_match_bracket_class.lua",
        vec![Value::integer(2), Value::integer(49), Value::integer(50)],
    );
    assert_success_fixture(
        "stdlib/string_match_negated_bracket_class.lua",
        vec![Value::integer(2), Value::integer(49), Value::integer(50)],
    );
    assert_success_fixture(
        "stdlib/string_match_quantifiers.lua",
        vec![Value::integer(4), Value::integer(97), Value::integer(98)],
    );
    assert_success_fixture(
        "stdlib/string_match_optional_quantifier.lua",
        vec![Value::integer(2), Value::integer(97), Value::integer(98)],
    );
    assert_success_fixture(
        "stdlib/string_match_start_anchor_init.lua",
        vec![Value::integer(2), Value::integer(98), Value::integer(99)],
    );
    assert_success_fixture(
        "stdlib/string_match_end_anchor.lua",
        vec![Value::integer(2), Value::integer(98), Value::integer(99)],
    );
    assert_success_fixture(
        "stdlib/string_match_init.lua",
        vec![Value::integer(2), Value::integer(98)],
    );
    assert_success_fixture(
        "stdlib/string_match_missing.lua",
        vec![Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/string_gsub_limit.lua",
        vec![Value::integer(120), Value::integer(120), Value::integer(51)],
    );
    assert_success_fixture_values("stdlib/string_gsub_missing.lua", |actual| {
        assert_eq!(
            actual.len(),
            2,
            "string.gsub missing fixture should return string and count"
        );
        assert!(
            actual[0].is_string(),
            "string.gsub missing result should be a string: {actual:?}"
        );
        assert_eq!(
            actual[1],
            Value::integer(0),
            "string.gsub missing count should be zero"
        );
    });
    assert_success_fixture_values("stdlib/string_gsub_table_replacement.lua", |actual| {
        assert_eq!(
            actual.len(),
            2,
            "string.gsub table replacement should return string and count"
        );
        assert!(
            actual[0].is_string(),
            "string.gsub table replacement result should be a string: {actual:?}"
        );
        assert_eq!(
            actual[1],
            Value::integer(1),
            "string.gsub table replacement count should be one"
        );
    });
    assert_success_fixture_values("stdlib/string_gsub_function_replacement.lua", |actual| {
        assert_eq!(
            actual.len(),
            2,
            "string.gsub function replacement should return string and count"
        );
        assert!(
            actual[0].is_string(),
            "string.gsub function replacement result should be a string: {actual:?}"
        );
        assert_eq!(
            actual[1],
            Value::integer(1),
            "string.gsub function replacement count should be one"
        );
    });
    assert_success_fixture(
        "stdlib/string_gmatch_positions.lua",
        vec![Value::integer(14)],
    );
    assert_success_fixture(
        "stdlib/string_gmatch_callable.lua",
        vec![
            Value::integer(1),
            Value::integer(2),
            Value::integer(3),
        ],
    );
    assert_success_fixture(
        "stdlib/string_gmatch_start_anchor.lua",
        vec![Value::integer(188)],
    );
    assert_success_fixture(
        "stdlib/string_gmatch_empty_matches.lua",
        vec![Value::integer(3)],
    );
    assert_success_fixture("stdlib/string_rep_empty.lua", vec![Value::integer(0)]);
    assert_success_fixture("stdlib/string_reverse_empty.lua", vec![Value::integer(0)]);
    assert_success_fixture(
        "stdlib/string_sub_default_end.lua",
        vec![Value::integer(2), Value::integer(99), Value::integer(100)],
    );
    assert_success_fixture("stdlib/string_sub_empty.lua", vec![Value::integer(0)]);
    assert_success_fixture(
        "stdlib/string_pattern_advanced.lua",
        vec![Value::integer(7), Value::integer(3)],
    );
    assert_success_fixture(
        "stdlib/string_format.lua",
        vec![
            Value::integer(7),
            Value::integer(48),
            Value::integer(102),
            Value::integer(58),
            Value::integer(43),
        ],
    );
    assert_success_fixture(
        "stdlib/string_format_char.lua",
        vec![
            Value::integer(3),
            Value::integer(65),
            Value::integer(58),
            Value::integer(66),
        ],
    );
    assert_success_fixture(
        "stdlib/string_format_float.lua",
        vec![
            Value::integer(9),
            Value::integer(46),
            Value::integer(58),
            Value::integer(43),
        ],
    );
    assert_success_fixture(
        "stdlib/string_format_quote.lua",
        vec![
            Value::integer(4),
            Value::integer(34),
            Value::integer(97),
            Value::integer(98),
            Value::integer(34),
        ],
    );
    assert_success_fixture(
        "stdlib/string_format_quote_scalars.lua",
        vec![
            Value::integer(15),
            Value::integer(110),
            Value::integer(58),
            Value::integer(116),
            Value::integer(58),
            Value::integer(45),
            Value::integer(58),
            Value::integer(53),
        ],
    );
    assert_success_fixture(
        "stdlib/string_format_percent.lua",
        vec![
            Value::integer(5),
            Value::integer(97),
            Value::integer(37),
            Value::integer(98),
            Value::integer(37),
            Value::integer(37),
        ],
    );
    assert_success_fixture_values("stdlib/string_format_pointer.lua", |actual| {
        assert_eq!(actual.len(), 1, "string.format %p should return one value");
        assert!(
            actual[0].is_string(),
            "string.format %p result should be a string: {actual:?}"
        );
    });
    assert_success_fixture(
        "stdlib/string_format_long_strings.lua",
        vec![Value::integer(50), Value::integer(107)],
    );
    assert_success_fixture(
        "stdlib/string_format_string_modifiers.lua",
        vec![
            Value::integer(10),
            Value::integer(32),
            Value::integer(32),
            Value::integer(97),
            Value::integer(58),
            Value::integer(32),
            Value::integer(32),
        ],
    );
    assert_success_fixture(
        "stdlib/string_format_integer_modifiers.lua",
        vec![
            Value::integer(9),
            Value::integer(55),
            Value::integer(32),
            Value::integer(32),
            Value::integer(58),
            Value::integer(48),
            Value::integer(55),
        ],
    );
    assert_success_fixture(
        "stdlib/string_format_integer_alternate.lua",
        vec![
            Value::integer(8),
            Value::integer(48),
            Value::integer(120),
            Value::integer(102),
            Value::integer(58),
            Value::integer(48),
            Value::integer(48),
        ],
    );
    assert_success_fixture(
        "stdlib/string_format_signed_flags.lua",
        vec![
            Value::integer(11),
            Value::integer(43),
            Value::integer(58),
            Value::integer(32),
            Value::integer(43),
            Value::integer(48),
            Value::integer(55),
        ],
    );
    assert_success_fixture(
        "stdlib/string_format_integer_precision.lua",
        vec![
            Value::integer(7),
            Value::integer(48),
            Value::integer(55),
            Value::integer(58),
            Value::integer(48),
            Value::integer(97),
        ],
    );
    assert_success_fixture(
        "stdlib/string_ops.lua",
        vec![
            Value::integer(8),
            Value::integer(4),
            Value::integer(99),
            Value::integer(90),
            Value::integer(97),
        ],
    );
    assert_success_fixture(
        "stdlib/string_case_empty.lua",
        vec![Value::integer(0), Value::integer(0)],
    );
    assert_success_fixture(
        "stdlib/string_byte_char.lua",
        vec![
            Value::integer(3),
            Value::integer(65),
            Value::integer(66),
            Value::integer(67),
            Value::integer(65),
        ],
    );
    assert_success_fixture("stdlib/string_char_empty.lua", vec![Value::integer(0)]);
    assert_success_fixture(
        "stdlib/string_byte_out_of_range.lua",
        vec![Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/string_byte_range.lua",
        vec![Value::integer(66), Value::integer(67)],
    );
    assert_success_fixture(
        "stdlib/math_numeric.lua",
        vec![
            Value::integer(3),
            Value::integer(4),
            Value::float(9.0),
            Value::float(3.0),
            Value::float(8.0),
            Value::integer(12),
            Value::integer(102),
            Value::boolean(false),
        ],
    );
    assert_success_fixture(
        "stdlib/math_integer_rounding.lua",
        vec![Value::integer(9), Value::integer(9)],
    );
    assert_success_fixture("stdlib/math_log_identity.lua", vec![Value::float(0.0)]);
    assert_success_fixture("stdlib/math_log_base10.lua", vec![Value::float(2.0)]);
    assert_success_fixture("stdlib/math_sqrt_zero.lua", vec![Value::float(0.0)]);
    assert_success_fixture(
        "stdlib/math_trig.lua",
        vec![
            Value::float(1.0),
            Value::float(0.0),
            Value::float(180.0),
            Value::float(1.0),
        ],
    );
    assert_success_fixture(
        "stdlib/math_angle_zero.lua",
        vec![Value::float(0.0), Value::float(0.0)],
    );
    assert_success_fixture(
        "stdlib/math_decompose.lua",
        vec![Value::float(0.75), Value::integer(4)],
    );
    assert_success_fixture(
        "stdlib/math_frexp_zero.lua",
        vec![Value::float(0.0), Value::integer(0)],
    );
    assert_success_fixture("stdlib/math_ldexp_negative.lua", vec![Value::float(0.5)]);
    assert_success_fixture(
        "stdlib/math_modf.lua",
        vec![Value::integer(-3), Value::float(-0.25)],
    );
    assert_success_fixture(
        "stdlib/math_modf_integer.lua",
        vec![Value::integer(5), Value::float(0.0)],
    );
    assert_success_fixture(
        "stdlib/math_nil_results.lua",
        vec![Value::boolean(true), Value::boolean(true), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/math_type_subtypes.lua",
        vec![
            Value::integer(7),
            Value::integer(105),
            Value::integer(114),
            Value::integer(5),
            Value::integer(102),
            Value::integer(116),
        ],
    );
    assert_success_fixture(
        "stdlib/math_type_nil.lua",
        vec![
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/math_constants.lua",
        vec![
            Value::integer(-1),
            Value::integer(102),
            Value::integer(102),
        ],
    );
    assert_success_fixture("stdlib/math_tointeger_float.lua", vec![Value::integer(7)]);
    assert_success_fixture(
        "stdlib/math_tointeger_integer.lua",
        vec![Value::integer(-12)],
    );
    assert_success_fixture(
        "stdlib/math_tointeger_nil.lua",
        vec![
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/math_minmax_float.lua",
        vec![Value::float(1.5), Value::integer(7)],
    );
    assert_success_fixture("stdlib/math_fmod_negative.lua", vec![Value::integer(-2)]);
    assert_success_fixture("stdlib/math_fmod_float.lua", vec![Value::float(1.5)]);
    assert_success_fixture("stdlib/math_ult_false.lua", vec![Value::boolean(false)]);
    assert_success_fixture("stdlib/math_ult_true.lua", vec![Value::boolean(true)]);
    assert_success_fixture(
        "stdlib/math_random.lua",
        vec![Value::integer(1), Value::integer(7), Value::integer(9)],
    );
    assert_success_fixture(
        "stdlib/math_random_modes.lua",
        vec![Value::integer(102), Value::integer(105), Value::integer(114)],
    );
    assert_success_fixture(
        "stdlib/table_string_utf8.lua",
        vec![
            Value::integer(1),
            Value::integer(2),
            Value::integer(3),
            Value::integer(4),
            Value::integer(3),
            Value::integer(2),
            Value::integer(66),
        ],
    );
    assert_success_fixture(
        "stdlib/table_mutation.lua",
        vec![
            Value::integer(5),
            Value::integer(97),
            Value::integer(98),
            Value::integer(20),
            Value::integer(30),
            Value::integer(40),
            Value::integer(40),
            Value::integer(8),
        ],
    );
    assert_success_fixture(
        "stdlib/table_insert_append.lua",
        vec![Value::integer(1), Value::integer(2)],
    );
    assert_success_fixture(
        "stdlib/table_insert_first.lua",
        vec![Value::integer(97), Value::integer(98), Value::integer(99)],
    );
    assert_success_fixture(
        "stdlib/table_insert_position.lua",
        vec![Value::integer(97), Value::integer(98), Value::integer(99)],
    );
    assert_success_fixture("stdlib/table_remove_empty.lua", vec![Value::boolean(true)]);
    assert_success_fixture(
        "stdlib/table_remove_default.lua",
        vec![Value::integer(2), Value::integer(1), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/table_remove_first.lua",
        vec![
            Value::integer(10),
            Value::integer(20),
            Value::integer(30),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/table_remove_position.lua",
        vec![
            Value::integer(20),
            Value::integer(10),
            Value::integer(30),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/table_move_overlap.lua",
        vec![Value::integer(1), Value::integer(1), Value::integer(2)],
    );
    assert_success_fixture(
        "stdlib/table_move_destination.lua",
        vec![
            Value::boolean(true),
            Value::integer(2),
            Value::integer(3),
            Value::integer(1),
            Value::integer(2),
            Value::integer(3),
        ],
    );
    assert_success_fixture(
        "stdlib/table_move_empty.lua",
        vec![
            Value::boolean(true),
            Value::integer(1),
            Value::integer(2),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/table_pack_nil.lua",
        vec![
            Value::integer(3),
            Value::integer(1),
            Value::boolean(true),
            Value::integer(3),
        ],
    );
    assert_success_fixture(
        "stdlib/table_pack_empty.lua",
        vec![Value::integer(0), Value::boolean(true)],
    );
    assert_success_fixture("stdlib/table_concat_empty.lua", vec![Value::integer(0)]);
    assert_success_fixture(
        "stdlib/table_concat_default_separator.lua",
        vec![
            Value::integer(3),
            Value::integer(97),
            Value::integer(98),
            Value::integer(99),
        ],
    );
    assert_success_fixture(
        "stdlib/table_concat_default.lua",
        vec![
            Value::integer(5),
            Value::integer(97),
            Value::integer(45),
            Value::integer(99),
        ],
    );
    assert_success_fixture(
        "stdlib/table_concat_long_strings.lua",
        vec![Value::integer(251), Value::integer(33), Value::integer(124)],
    );
    assert_success_fixture(
        "stdlib/table_ranges.lua",
        vec![
            Value::integer(3),
            Value::integer(98),
            Value::integer(20),
            Value::integer(30),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/table_unpack.lua",
        vec![Value::integer(6), Value::integer(7)],
    );
    assert_success_fixture(
        "stdlib/table_unpack_default_bounds.lua",
        vec![Value::integer(1), Value::integer(2), Value::integer(3)],
    );
    assert_success_fixture(
        "stdlib/table_unpack_nil.lua",
        vec![Value::integer(1), Value::nil(), Value::integer(3)],
    );
    assert_success_fixture("stdlib/table_unpack_empty.lua", Vec::new());
    assert_success_fixture(
        "stdlib/table_sort.lua",
        vec![Value::integer(1), Value::integer(2), Value::integer(3)],
    );
    assert_success_fixture(
        "stdlib/table_sort_strings.lua",
        vec![Value::integer(97), Value::integer(98), Value::integer(99)],
    );
    assert_success_fixture(
        "stdlib/table_sort_long_strings.lua",
        vec![
            Value::integer(97),
            Value::integer(50),
            Value::integer(98),
            Value::integer(50),
        ],
    );
    assert_success_fixture("stdlib/table_sort_single.lua", vec![Value::integer(42)]);
    assert_success_fixture(
        "stdlib/table_sort_comparator.lua",
        vec![Value::integer(2), Value::integer(1)],
    );
    assert_success_fixture(
        "stdlib/io_stubs.lua",
        vec![
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture_values("stdlib/io_open_result.lua", |actual| {
        assert_eq!(actual.len(), 2, "io.open should return nil plus message");
        assert_eq!(actual[0], Value::nil(), "io.open result should be nil");
        assert!(
            actual[1].is_string(),
            "io.open message should be a string: {actual:?}"
        );
    });
    assert_success_fixture_values("stdlib/io_tmpfile_result.lua", |actual| {
        assert_eq!(
            actual.len(),
            2,
            "io.tmpfile should return nil plus message"
        );
        assert_eq!(actual[0], Value::nil(), "io.tmpfile result should be nil");
        assert!(
            actual[1].is_string(),
            "io.tmpfile message should be a string: {actual:?}"
        );
    });
    assert_success_fixture_values("stdlib/io_write_result.lua", |actual| {
        assert_eq!(
            actual.len(),
            2,
            "io.write should return nil plus message"
        );
        assert_eq!(actual[0], Value::nil(), "io.write result should be nil");
        assert!(
            actual[1].is_string(),
            "io.write message should be a string: {actual:?}"
        );
    });
    assert_success_fixture_values("stdlib/io_flush_result.lua", |actual| {
        assert_eq!(
            actual.len(),
            2,
            "io.flush should return nil plus message"
        );
        assert_eq!(actual[0], Value::nil(), "io.flush result should be nil");
        assert!(
            actual[1].is_string(),
            "io.flush message should be a string: {actual:?}"
        );
    });
    assert_success_fixture(
        "stdlib/io_type.lua",
        vec![Value::boolean(true), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/utf8_iteration.lua",
        vec![
            Value::integer(198),
            Value::integer(3),
            Value::integer(2),
            Value::integer(3),
            Value::integer(91),
        ],
    );
    assert_success_fixture(
        "stdlib/utf8_char.lua",
        vec![Value::integer(1), Value::integer(65)],
    );
    assert_success_fixture("stdlib/utf8_char_empty.lua", vec![Value::integer(0)]);
    assert_success_fixture(
        "stdlib/utf8_char_max.lua",
        vec![
            Value::integer(4),
            Value::integer(1),
            Value::integer(1114111),
        ],
    );
    assert_success_fixture("stdlib/utf8_codepoint_empty.lua", Vec::new());
    assert_success_fixture(
        "stdlib/utf8_codepoint_range.lua",
        vec![Value::integer(66), Value::integer(67)],
    );
    assert_success_fixture(
        "stdlib/utf8_codepoint_multibyte_range.lua",
        vec![Value::integer(65), Value::integer(233)],
    );
    assert_success_fixture(
        "stdlib/utf8_multibyte.lua",
        vec![
            Value::integer(6),
            Value::integer(2),
            Value::integer(119070),
            Value::integer(3),
        ],
    );
    assert_success_fixture(
        "stdlib/utf8_long_strings.lua",
        vec![
            Value::integer(50),
            Value::integer(50),
            Value::integer(97),
            Value::integer(50),
            Value::integer(50),
        ],
    );
    assert_success_fixture(
        "stdlib/utf8_offset_missing.lua",
        vec![Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/utf8_offset_backward_multibyte.lua",
        vec![Value::integer(3), Value::integer(3)],
    );
    assert_success_fixture(
        "stdlib/utf8_offset_containing.lua",
        vec![Value::integer(2), Value::integer(3)],
    );
    assert_success_fixture("stdlib/utf8_len_empty.lua", vec![Value::integer(0)]);
    assert_success_fixture("stdlib/utf8_len_bounds.lua", vec![Value::integer(2)]);
    assert_success_fixture("stdlib/utf8_len_relative.lua", vec![Value::integer(3)]);
    assert_success_fixture(
        "stdlib/os_package_debug.lua",
        vec![
            Value::float(6.0),
            Value::integer(115),
            Value::integer(116),
            Value::integer(115),
        ],
    );
    assert_success_fixture(
        "stdlib/package_require.lua",
        vec![
            Value::integer(77),
            Value::integer(77),
            Value::integer(77),
            Value::integer(23),
        ],
    );
    assert_success_fixture(
        "stdlib/package_require_nil_loader.lua",
        vec![Value::boolean(true), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/package_preload_searcher.lua",
        vec![Value::integer(42)],
    );
    assert_success_fixture_values("stdlib/package_searchpath.lua", |actual| {
        assert_eq!(
            actual.len(),
            2,
            "package.searchpath should return nil plus message"
        );
        assert_eq!(
            actual[0],
            Value::nil(),
            "package.searchpath path should be nil"
        );
        assert!(
            actual[1].is_string(),
            "package.searchpath miss message should be a string: {actual:?}"
        );
    });
    assert_success_fixture(
        "stdlib/package_searchpath_found.lua",
        vec![Value::integer(12), Value::integer(46), Value::integer(67)],
    );
    assert_success_fixture(
        "stdlib/package_config.lua",
        vec![Value::integer(115), Value::integer(10)],
    );
    assert_success_fixture_values("stdlib/package_loadlib.lua", |actual| {
        assert_eq!(
            actual.len(),
            3,
            "package.loadlib should return nil plus message plus stage"
        );
        assert_eq!(
            actual[0],
            Value::nil(),
            "package.loadlib result should be nil"
        );
        assert!(
            actual[1].is_string(),
            "package.loadlib message should be a string: {actual:?}"
        );
        assert!(
            actual[2].is_string(),
            "package.loadlib stage should be a string: {actual:?}"
        );
    });
    assert_success_fixture(
        "stdlib/package_c_searchers.lua",
        vec![
            Value::integer(110),
            Value::integer(48),
            Value::integer(110),
            Value::integer(42),
        ],
    );
    assert_success_fixture(
        "stdlib/debug_introspection.lua",
        vec![
            Value::integer(76),
            Value::integer(0),
            Value::boolean(false),
            Value::integer(109),
            Value::integer(0),
            Value::boolean(false),
            Value::boolean(false),
            Value::integer(0),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/debug_registry.lua",
        vec![Value::integer(42), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/debug_upvalues.lua",
        vec![
            Value::integer(10),
            Value::integer(30),
            Value::integer(40),
            Value::integer(40),
            Value::boolean(false),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/debug_locals.lua",
        vec![
            Value::integer(43),
            Value::integer(120),
            Value::integer(120),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/debug_traceback.lua",
        vec![Value::integer(98), Value::integer(115)],
    );
    assert_success_fixture(
        "stdlib/debug_metatable.lua",
        vec![
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/debug_uservalue.lua",
        vec![Value::boolean(true), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/debug_hooks.lua",
        vec![
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/os_time_date.lua",
        vec![
            Value::integer(1970),
            Value::integer(1),
            Value::integer(1),
            Value::integer(0),
            Value::integer(0),
            Value::integer(0),
            Value::float(86400.0),
            Value::integer(115),
        ],
    );
    assert_success_fixture(
        "stdlib/os_time_normalize.lua",
        vec![
            Value::integer(31579200),
            Value::integer(1971),
            Value::integer(1),
            Value::integer(1),
            Value::integer(12),
            Value::integer(0),
            Value::integer(0),
        ],
    );
    assert_success_fixture(
        "stdlib/os_time_defaults.lua",
        vec![
            Value::integer(0),
            Value::integer(0),
            Value::integer(0),
            Value::integer(0),
        ],
    );
    assert_success_fixture(
        "stdlib/os_date_format.lua",
        vec![
            Value::integer(19),
            Value::integer(49),
            Value::integer(32),
            Value::integer(48),
        ],
    );
    assert_success_fixture(
        "stdlib/os_date_table_fields.lua",
        vec![Value::integer(5), Value::integer(1), Value::boolean(false)],
    );
    assert_success_fixture(
        "stdlib/os_date_names.lua",
        vec![
            Value::integer(44),
            Value::integer(84),
            Value::integer(84),
            Value::integer(74),
            Value::integer(74),
        ],
    );
    assert_success_fixture(
        "stdlib/os_date_ordinals.lua",
        vec![
            Value::integer(27),
            Value::integer(48),
            Value::integer(52),
            Value::integer(37),
        ],
    );
    assert_success_fixture(
        "stdlib/os_locale.lua",
        vec![Value::integer(67), Value::integer(67), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/os_locale_categories.lua",
        vec![
            Value::integer(67),
            Value::integer(67),
            Value::integer(67),
            Value::integer(67),
            Value::integer(67),
            Value::integer(67),
        ],
    );
    assert_success_fixture("stdlib/os_execute.lua", vec![Value::boolean(true)]);
    assert_success_fixture_values("stdlib/os_execute_status.lua", |actual| {
        assert_eq!(
            actual.len(),
            3,
            "os.execute command should return status tuple"
        );
        assert_eq!(actual[0], Value::boolean(true), "os.execute should succeed");
        assert!(
            actual[1].is_string(),
            "os.execute status label should be a string: {actual:?}"
        );
        assert_eq!(actual[2], Value::integer(0), "os.execute code should be 0");
    });
    assert_success_fixture_values("stdlib/os_execute_failure.lua", |actual| {
        assert_eq!(
            actual.len(),
            3,
            "failed os.execute command should return status tuple"
        );
        assert_eq!(actual[0], Value::nil(), "os.execute should report failure");
        assert!(
            actual[1].is_string(),
            "os.execute status label should be a string: {actual:?}"
        );
        assert_eq!(actual[2], Value::integer(7), "os.execute code should be 7");
    });
    assert_success_fixture("stdlib/os_clock.lua", vec![Value::integer(110)]);
    assert_success_fixture("stdlib/os_tmpname.lua", vec![Value::integer(115)]);
    assert_success_fixture_values("stdlib/os_tmpname_result.lua", |actual| {
        assert_eq!(actual.len(), 1, "os.tmpname should return one filename");
        assert!(
            actual[0].is_string(),
            "os.tmpname result should be a string: {actual:?}"
        );
    });
    assert_success_fixture("stdlib/os_getenv.lua", vec![Value::boolean(true)]);
    assert_success_fixture_values("stdlib/os_remove.lua", |actual| {
        assert_eq!(
            actual.len(),
            3,
            "os.remove absent path should return nil plus message plus code"
        );
        assert_eq!(
            actual[0],
            Value::nil(),
            "os.remove absent path result should be nil"
        );
        assert!(
            actual[1].is_string(),
            "os.remove message should be a string: {actual:?}"
        );
        assert!(
            actual[2].as_integer().is_some_and(|code| code != 0),
            "os.remove code should be non-zero: {actual:?}"
        );
    });
    assert_success_fixture_values("stdlib/os_rename.lua", |actual| {
        assert_eq!(
            actual.len(),
            3,
            "os.rename absent path should return nil plus message plus code"
        );
        assert_eq!(
            actual[0],
            Value::nil(),
            "os.rename absent path result should be nil"
        );
        assert!(
            actual[1].is_string(),
            "os.rename message should be a string: {actual:?}"
        );
        assert!(
            actual[2].as_integer().is_some_and(|code| code != 0),
            "os.rename code should be non-zero: {actual:?}"
        );
    });
}

#[test]
fn conformance_error_fixtures() {
    assert_error_fixture("errors/non_callable.lua");
    assert_error_fixture("errors/base_error.lua");
    assert_error_fixture("errors/syntax_unclosed.lua");
    assert_error_fixture("errors/bad_argument.lua");
    assert_error_fixture("errors/non_table_index.lua");
    assert_error_fixture("errors/arithmetic_type.lua");
    assert_error_fixture("errors/debug_uservalue.lua");
    assert_error_fixture("errors/string_format_unsupported.lua");
}

#[test]
fn conformance_coroutine_fixtures() {
    assert_success_fixture("coroutine/wrap.lua", vec![Value::integer(42)]);
    assert_success_fixture("coroutine/resume_status.lua", vec![Value::boolean(true)]);
    assert_success_fixture(
        "coroutine/lifecycle.lua",
        vec![
            Value::integer(115),
            Value::boolean(true),
            Value::boolean(true),
            Value::integer(100),
        ],
    );
    assert_success_fixture("coroutine/close.lua", vec![Value::boolean(true)]);
}

fn assert_success_fixture(path: &str, expected: Vec<Value>) {
    assert_success_fixture_values(path, |actual| {
        assert_eq!(actual, expected, "fixture values mismatch for {path}");
    });
}

fn assert_success_fixture_values(path: &str, assert_values: impl FnOnce(Vec<Value>)) {
    let source = fs::read_to_string(fixture_path(path)).expect("fixture should be readable");
    let actual = Lua::new()
        .eval(source)
        .unwrap_or_else(|error| panic!("fixture {path} should succeed: {error:?}"));
    assert_values(actual);
}

fn assert_error_fixture(path: &str) {
    let source = fs::read_to_string(fixture_path(path)).expect("fixture should be readable");
    let result = Lua::new().eval(source);
    assert!(result.is_err(), "fixture {path} should error: {result:?}");
}

fn fixture_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance")
        .join(path)
}
