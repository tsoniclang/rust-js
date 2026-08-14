use tsonic_rust_js::date::JsDate;

#[test]
fn date_clones_preserve_object_identity() {
    let date = JsDate::from_millis(0.0);
    let alias = date.clone();
    let distinct = JsDate::from_millis(0.0);

    assert_eq!(date, alias);
    assert_ne!(date, distinct);
    assert_eq!(date.get_time(), distinct.get_time());
}

#[test]
fn date_epoch_and_iso_roundtrip() {
    let date = JsDate::from_millis(0.0);
    assert_eq!(date.get_time(), 0.0);
    assert_eq!(date.value_of(), 0.0);
    assert_eq!(date.to_iso_string().unwrap(), "1970-01-01T00:00:00.000Z");
    assert_eq!(date.to_json(), "1970-01-01T00:00:00.000Z");
    assert_eq!(JsDate::parse("1970-01-01T00:00:00.000Z"), 0.0);
}

#[test]
fn date_string_constructor_uses_the_same_exact_parse_contract() {
    let date = JsDate::from_string("1970-01-02T00:00:00.000Z");
    assert_eq!(date.get_time(), 86_400_000.0);
    assert!(JsDate::from_string("not-a-date").get_time().is_nan());
}

#[test]
fn date_supports_common_utc_iso_values() {
    let date = JsDate::from_millis(JsDate::parse("2020-02-29T12:34:56.789Z"));
    assert_eq!(date.to_iso_string().unwrap(), "2020-02-29T12:34:56.789Z");
    assert_eq!(date.get_utc_full_year().unwrap(), 2020);
    assert_eq!(date.get_utc_month().unwrap(), 1);
    assert_eq!(date.get_utc_date().unwrap(), 29);
    assert_eq!(date.get_utc_hours().unwrap(), 12);
    assert_eq!(date.get_utc_minutes().unwrap(), 34);
    assert_eq!(date.get_utc_seconds().unwrap(), 56);
    assert_eq!(date.get_utc_milliseconds().unwrap(), 789);
}

#[test]
fn date_parse_accepts_the_deterministic_iso_subset() {
    // Values checked against Node's Date.parse.
    assert_eq!(JsDate::parse("2020-01-02"), 1_577_923_200_000.0);
    assert_eq!(JsDate::parse("2020-01-02T03:04:05Z"), 1_577_934_245_000.0);
    assert_eq!(
        JsDate::parse("2020-01-02T03:04:05.678Z"),
        1_577_934_245_678.0
    );
    assert_eq!(JsDate::parse("2020-01-02T03:04:05.6Z"), 1_577_934_245_600.0);
    assert_eq!(
        JsDate::parse("2020-01-02T03:04:05+05:30"),
        1_577_914_445_000.0
    );
    assert_eq!(
        JsDate::parse("2020-01-02T03:04:05-08:00"),
        1_577_963_045_000.0
    );
    assert_eq!(JsDate::parse("0001-01-01"), -62_135_596_800_000.0);
    assert_eq!(
        JsDate::parse("9999-12-31T23:59:59.999Z"),
        253_402_300_799_999.0
    );
}

#[test]
fn date_parse_rejects_everything_else_with_nan() {
    // Everything outside the documented deterministic subset is NaN here,
    // including strings Node's legacy parser accepts with local-time (and
    // therefore machine-dependent) interpretation, such as "2020-1-02" or a
    // date-time without a timezone designator.
    for rejected in [
        "",
        "not a date",
        "1234",
        "2020-1-02",
        "2020-01-2",
        "2020-13-01",
        "2020-00-10",
        "2021-02-29",
        "2021-04-31",
        "2020-01-02T03:04Z",
        "2020-01-02T03:04:05",
        "2020-01-02T24:00:00Z",
        "2020-01-02T03:60:05Z",
        "2020-01-02T03:04:60Z",
        "2020-01-02T03:04:05.1234Z",
        "2020-01-02T03:04:05+0530",
        "2020-01-02T03:04:05+24:00",
        " 2020-01-02",
        "2020-01-02 ",
        "2020/01/02",
        "20💚-01-01",
        "2020-01-01T0💚:00:00Z",
        "2020-01-01T00:00:00+0💚0",
    ] {
        assert!(
            JsDate::parse(rejected).is_nan(),
            "expected NaN for {rejected:?}"
        );
    }
}

#[test]
fn date_utc_matches_js_overflow_and_clipping() {
    // Values checked against Node's Date.UTC.
    assert_eq!(
        JsDate::utc(2020.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0),
        1_577_836_800_000.0
    );
    assert_eq!(
        JsDate::utc(2020.0, 1.0, 29.0, 12.0, 34.0, 56.0, 789.0),
        1_582_979_696_789.0
    );
    // Month/day overflow carries.
    assert_eq!(
        JsDate::utc(2020.0, 12.0, 1.0, 0.0, 0.0, 0.0, 0.0),
        JsDate::utc(2021.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(
        JsDate::utc(2020.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        JsDate::utc(2019.0, 10.0, 30.0, 0.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(
        JsDate::utc(2020.0, 0.0, 1.0, 25.0, 61.0, 61.0, 1001.0),
        1_577_930_522_001.0
    );
    // Two-digit years map into 1900..=1999.
    assert_eq!(
        JsDate::utc(99.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0),
        915_148_800_000.0
    );
    // Fractions truncate toward zero.
    assert_eq!(
        JsDate::utc(2020.5, 0.9, 1.7, 0.0, 0.0, 0.0, 0.5),
        1_577_836_800_000.0
    );
    // Non-finite arguments and out-of-range results are NaN.
    assert!(JsDate::utc(f64::NAN, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0).is_nan());
    assert!(JsDate::utc(f64::INFINITY, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0).is_nan());
    assert!(JsDate::utc(275_760.0, 8.0, 14.0, 0.0, 0.0, 0.0, 0.0).is_nan());
    assert_eq!(
        JsDate::utc(275_760.0, 8.0, 13.0, 0.0, 0.0, 0.0, 0.0),
        8.64e15
    );
    assert_eq!(
        JsDate::utc(-271_821.0, 3.0, 20.0, 0.0, 0.0, 0.0, 0.0),
        -8.64e15
    );
}

#[test]
fn date_to_json_serializes_invalid_dates_as_null() {
    assert_eq!(JsDate::from_millis(f64::NAN).to_json(), "null");
    assert_eq!(
        JsDate::from_millis(86_400_000.0).to_json(),
        "1970-01-02T00:00:00.000Z"
    );
}

#[test]
fn date_now_is_finite_and_monotonic_enough() {
    let before = JsDate::now();
    let constructed = JsDate::new();
    let after = JsDate::now();
    assert!(before.is_finite());
    assert!(constructed.get_time() >= before);
    assert!(constructed.get_time() <= after);
    assert!(after >= before);
}
