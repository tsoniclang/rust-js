use std::process::Command;
use tsonic_rust_js::date::JsDate;

const INSTANTS: [f64; 20] = [
    -8_640_000_000_000_000.0,
    -62_167_219_200_000.0,
    -2_208_988_800_000.0,
    -1.0,
    0.0,
    1.0,
    946_684_800_001.0,
    1_262_304_000_789.0,
    1_325_239_199_999.0,
    1_325_239_200_000.0,
    1_582_978_496_789.0,
    1_583_650_799_999.0,
    1_583_650_800_000.0,
    1_604_210_399_999.0,
    1_604_210_400_000.0,
    1_625_097_600_000.0,
    253_402_300_799_999.0,
    8_210_266_876_800_000.0,
    8_639_999_999_999_999.0,
    8_640_000_000_000_000.0,
];

fn parts(date: &JsDate) -> [f64; 9] {
    [
        date.get_full_year().unwrap(),
        date.get_month().unwrap(),
        date.get_date().unwrap(),
        date.get_day().unwrap(),
        date.get_hours().unwrap(),
        date.get_minutes().unwrap(),
        date.get_seconds().unwrap(),
        date.get_milliseconds(),
        date.get_timezone_offset().unwrap(),
    ]
}

fn observations(output: &[u8]) -> Vec<Vec<f64>> {
    String::from_utf8(output.to_vec())
        .unwrap()
        .lines()
        .filter_map(|line| line.strip_prefix("LOCAL_DATE:"))
        .map(|line| {
            line.split(',')
                .map(|value| value.parse().unwrap())
                .collect()
        })
        .collect()
}

#[test]
fn local_date_matches_node_in_isolated_timezones() {
    if std::env::var_os("TSONIC_LOCAL_DATE_CHILD").is_some() {
        for instant in INSTANTS {
            let values = parts(&JsDate::from_millis(instant));
            println!(
                "LOCAL_DATE:{}",
                values.map(|value| value.to_string()).join(",")
            );
        }
        for invalid in [f64::NAN, f64::INFINITY, -f64::INFINITY, 8.64e15 + 1.0] {
            assert!(parts(&JsDate::from_millis(invalid))
                .iter()
                .all(|value| value.is_nan()));
        }
        let retained = JsDate::from_millis(0.0);
        std::env::set_var("TZ", "UTC");
        assert_eq!(retained.get_hours().unwrap(), 0.0);
        std::env::set_var("TZ", "America/New_York");
        assert_eq!(retained.get_hours().unwrap(), 19.0);
        assert_eq!(retained.get_time(), 0.0);
        std::env::set_var("TZ", "Tsonic/DefinitelyMissingZone");
        assert!(retained.get_hours().is_err());
        assert!(JsDate::from_millis(f64::NAN).get_hours().unwrap().is_nan());
        return;
    }
    let source = format!(
        "for (const instant of {INSTANTS:?}) {{ const date = new Date(instant); \
         console.log('LOCAL_DATE:' + [date.getFullYear(), date.getMonth(), \
         date.getDate(), date.getDay(), date.getHours(), date.getMinutes(), \
         date.getSeconds(), date.getMilliseconds(), date.getTimezoneOffset()].join(',')); }}"
    );
    let mut mismatches = Vec::new();
    for zone in [
        "",
        "UTC",
        "America/New_York",
        "Europe/Berlin",
        "Asia/Kolkata",
        "Asia/Kathmandu",
        "Australia/Lord_Howe",
        "Pacific/Apia",
    ] {
        let expected = Command::new("node")
            .args(["--input-type=module", "-e", &source])
            .env("TZ", zone)
            .output()
            .unwrap();
        assert!(expected.status.success(), "{expected:?}");
        let actual = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "local_date_tests::local_date_matches_node_in_isolated_timezones",
                "--nocapture",
            ])
            .env("TZ", zone)
            .env("TSONIC_LOCAL_DATE_CHILD", "1")
            .output()
            .unwrap();
        assert!(actual.status.success(), "{actual:?}");
        let expected = observations(&expected.stdout);
        let actual = observations(&actual.stdout);
        assert_eq!(expected.len(), INSTANTS.len());
        assert_eq!(actual.len(), INSTANTS.len());
        for (index, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
            if actual != expected {
                mismatches.push(format!(
                    "TZ={zone}, instant={}: {actual:?} != {expected:?}",
                    INSTANTS[index]
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
