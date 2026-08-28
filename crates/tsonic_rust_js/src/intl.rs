//! Deterministic, closed internationalization carriers.

use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier};

use crate::array::JsArray;
use crate::errors::{range_error, type_error, JsResult};
use crate::value::JsValue;
use crate::JsDate;

const DEFAULT_LOCALE: &str = "en-US";
const DEFAULT_TIME_ZONE: &str = "UTC";

#[derive(Clone, Debug)]
pub struct IntlDateTimeFormatPart {
    part_type: String,
    value: String,
}

impl IntlDateTimeFormatPart {
    fn new(part_type: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            part_type: part_type.into(),
            value: value.into(),
        }
    }

    pub fn type_value(&self) -> String {
        self.part_type.clone()
    }

    pub fn value(&self) -> String {
        self.value.clone()
    }
}

#[derive(Clone, Debug)]
pub struct IntlNumberFormatPart {
    part_type: String,
    value: String,
}

impl IntlNumberFormatPart {
    fn new(part_type: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            part_type: part_type.into(),
            value: value.into(),
        }
    }

    pub fn type_value(&self) -> String {
        self.part_type.clone()
    }

    pub fn value(&self) -> String {
        self.value.clone()
    }
}

#[derive(Clone, Debug)]
pub struct IntlResolvedDateTimeFormatOptions {
    locale: String,
    time_zone: String,
}

impl IntlResolvedDateTimeFormatOptions {
    pub fn locale(&self) -> String {
        self.locale.clone()
    }

    pub fn calendar(&self) -> String {
        "gregory".to_owned()
    }

    pub fn numbering_system(&self) -> String {
        "latn".to_owned()
    }

    pub fn time_zone(&self) -> String {
        self.time_zone.clone()
    }
}

#[derive(Clone, Debug)]
pub struct IntlResolvedNumberFormatOptions {
    locale: String,
    style: String,
    minimum_integer_digits: u8,
    minimum_fraction_digits: u8,
    maximum_fraction_digits: u8,
    use_grouping: bool,
}

impl IntlResolvedNumberFormatOptions {
    pub fn locale(&self) -> String {
        self.locale.clone()
    }

    pub fn numbering_system(&self) -> String {
        "latn".to_owned()
    }

    pub fn style(&self) -> String {
        self.style.clone()
    }

    pub fn minimum_integer_digits(&self) -> f64 {
        f64::from(self.minimum_integer_digits)
    }

    pub fn minimum_fraction_digits(&self) -> f64 {
        f64::from(self.minimum_fraction_digits)
    }

    pub fn maximum_fraction_digits(&self) -> f64 {
        f64::from(self.maximum_fraction_digits)
    }

    pub fn use_grouping(&self) -> bool {
        self.use_grouping
    }
}

#[derive(Clone, Debug)]
pub struct IntlResolvedCollatorOptions {
    locale: String,
    usage: String,
    sensitivity: String,
    ignore_punctuation: bool,
    numeric: bool,
    case_first: String,
}

impl IntlResolvedCollatorOptions {
    pub fn locale(&self) -> String {
        self.locale.clone()
    }

    pub fn usage(&self) -> String {
        self.usage.clone()
    }

    pub fn sensitivity(&self) -> String {
        self.sensitivity.clone()
    }

    pub fn ignore_punctuation(&self) -> bool {
        self.ignore_punctuation
    }

    pub fn collation(&self) -> String {
        "default".to_owned()
    }

    pub fn numeric(&self) -> bool {
        self.numeric
    }

    pub fn case_first(&self) -> String {
        self.case_first.clone()
    }
}

#[derive(Clone, Debug)]
pub struct IntlDateTimeFormat {
    identity: ObjectIdentity,
    locale: String,
    time_zone: String,
    fields: DateTimeFields,
}

#[derive(Clone, Debug)]
struct DateTimeFields {
    weekday: Option<String>,
    era: Option<String>,
    year: Option<String>,
    month: Option<String>,
    day: Option<String>,
    hour: Option<String>,
    minute: Option<String>,
    second: Option<String>,
    time_zone_name: Option<String>,
    hour12: bool,
}

impl IntlDateTimeFormat {
    pub fn new() -> Self {
        Self::build(DEFAULT_LOCALE, &JsValue::Undefined).expect("default Intl.DateTimeFormat")
    }

    pub fn with_locale(locale: &str) -> JsResult<Self> {
        Self::build(locale, &JsValue::Undefined)
    }

    pub fn with_locales(locales: &JsArray<String>) -> JsResult<Self> {
        Self::with_locale(&first_locale(locales)?)
    }

    pub fn with_locale_options(locale: &str, options: &JsValue) -> JsResult<Self> {
        Self::build(locale, options)
    }

    pub fn with_locales_options(locales: &JsArray<String>, options: &JsValue) -> JsResult<Self> {
        Self::with_locale_options(&first_locale(locales)?, options)
    }

    pub fn format_default(&self) -> String {
        self.format_number(JsDate::now())
    }

    pub fn format_date(&self, value: &JsDate) -> String {
        self.format_number(value.get_time())
    }

    pub fn format_number(&self, value: f64) -> String {
        self.parts(value)
            .values()
            .into_iter()
            .flatten()
            .map(|part| part.value)
            .collect::<Vec<_>>()
            .concat()
    }

    pub fn format_to_parts_default(&self) -> JsArray<IntlDateTimeFormatPart> {
        self.parts(JsDate::now())
    }

    pub fn format_to_parts_date(&self, value: &JsDate) -> JsArray<IntlDateTimeFormatPart> {
        self.parts(value.get_time())
    }

    pub fn format_to_parts_number(&self, value: f64) -> JsArray<IntlDateTimeFormatPart> {
        self.parts(value)
    }

    pub fn resolved_options(&self) -> IntlResolvedDateTimeFormatOptions {
        IntlResolvedDateTimeFormatOptions {
            locale: self.locale.clone(),
            time_zone: self.time_zone.clone(),
        }
    }

    fn build(locale: &str, options: &JsValue) -> JsResult<Self> {
        let locale = canonical_locale(locale)?;
        validate_locale_matcher(options)?;
        let time_zone = string_option(options, "timeZone")?.unwrap_or_else(|| DEFAULT_TIME_ZONE.to_owned());
        if time_zone != DEFAULT_TIME_ZONE {
            return Err(range_error(format!(
                "Intl.DateTimeFormat supports only the deterministic '{DEFAULT_TIME_ZONE}' time zone"
            )));
        }
        let mut fields = DateTimeFields {
            weekday: enum_option(options, "weekday", &["long", "short", "narrow"])?,
            era: enum_option(options, "era", &["long", "short", "narrow"])?,
            year: enum_option(options, "year", &["numeric", "2-digit"])?,
            month: enum_option(options, "month", &["numeric", "2-digit", "long", "short", "narrow"])?,
            day: enum_option(options, "day", &["numeric", "2-digit"])?,
            hour: enum_option(options, "hour", &["numeric", "2-digit"])?,
            minute: enum_option(options, "minute", &["numeric", "2-digit"])?,
            second: enum_option(options, "second", &["numeric", "2-digit"])?,
            time_zone_name: enum_option(options, "timeZoneName", &["long", "short"])?,
            hour12: boolean_option(options, "hour12")?.unwrap_or(true),
        };
        if fields.weekday.is_none() && fields.era.is_none() && fields.year.is_none() && fields.month.is_none() &&
            fields.day.is_none() && fields.hour.is_none() && fields.minute.is_none() &&
            fields.second.is_none() && fields.time_zone_name.is_none()
        {
            fields.year = Some("numeric".to_owned());
            fields.month = Some("numeric".to_owned());
            fields.day = Some("numeric".to_owned());
        }
        Ok(Self {
            identity: ObjectIdentity::new(),
            locale,
            time_zone,
            fields,
        })
    }

    fn parts(&self, milliseconds: f64) -> JsArray<IntlDateTimeFormatPart> {
        if !milliseconds.is_finite() {
            return JsArray::from_dense(vec![IntlDateTimeFormatPart::new("literal", "Invalid Date")]);
        }
        let components = utc_components(milliseconds);
        let mut parts = Vec::new();
        if let Some(style) = &self.fields.weekday {
            parts.push(IntlDateTimeFormatPart::new(
                "weekday",
                weekday_name(components.weekday, style),
            ));
            parts.push(IntlDateTimeFormatPart::new("literal", ", "));
        }
        let mut date = Vec::new();
        if let Some(style) = &self.fields.month {
            date.push(IntlDateTimeFormatPart::new(
                "month",
                month_value(components.month, style),
            ));
        }
        if let Some(style) = &self.fields.day {
            date.push(IntlDateTimeFormatPart::new(
                "day",
                number_value(components.day, style),
            ));
        }
        if let Some(style) = &self.fields.year {
            date.push(IntlDateTimeFormatPart::new(
                "year",
                number_value(components.year, style),
            ));
        }
        append_separated(&mut parts, date, "/");
        if let Some(style) = &self.fields.era {
            if !parts.is_empty() {
                parts.push(IntlDateTimeFormatPart::new("literal", " "));
            }
            parts.push(IntlDateTimeFormatPart::new("era", era_value(style)));
        }
        let mut time = Vec::new();
        if let Some(style) = &self.fields.hour {
            let hour = if self.fields.hour12 {
                match components.hour % 12 {
                    0 => 12,
                    value => value,
                }
            } else {
                components.hour
            };
            time.push(IntlDateTimeFormatPart::new(
                "hour",
                number_value(hour, style),
            ));
        }
        if let Some(style) = &self.fields.minute {
            time.push(IntlDateTimeFormatPart::new(
                "minute",
                number_value(components.minute, style),
            ));
        }
        if let Some(style) = &self.fields.second {
            time.push(IntlDateTimeFormatPart::new(
                "second",
                number_value(components.second, style),
            ));
        }
        if !parts.is_empty() && !time.is_empty() {
            parts.push(IntlDateTimeFormatPart::new("literal", ", "));
        }
        append_separated(&mut parts, time, ":");
        if self.fields.hour.is_some() && self.fields.hour12 {
            parts.push(IntlDateTimeFormatPart::new("literal", " "));
            parts.push(IntlDateTimeFormatPart::new(
                "dayPeriod",
                if components.hour < 12 { "AM" } else { "PM" },
            ));
        }
        if let Some(style) = &self.fields.time_zone_name {
            if !parts.is_empty() {
                parts.push(IntlDateTimeFormatPart::new("literal", " "));
            }
            parts.push(IntlDateTimeFormatPart::new(
                "timeZoneName",
                if style == "long" { "Coordinated Universal Time" } else { "UTC" },
            ));
        }
        JsArray::from_dense(parts)
    }
}

impl Default for IntlDateTimeFormat {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectIdentityCarrier for IntlDateTimeFormat {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

#[derive(Clone, Debug)]
pub struct IntlNumberFormat {
    identity: ObjectIdentity,
    locale: String,
    style: String,
    currency_label: Option<String>,
    currency_display: String,
    use_grouping: bool,
    minimum_integer_digits: u8,
    minimum_fraction_digits: u8,
    maximum_fraction_digits: u8,
}

impl IntlNumberFormat {
    pub fn new() -> Self {
        Self::build(DEFAULT_LOCALE, &JsValue::Undefined).expect("default Intl.NumberFormat")
    }

    pub fn with_locale(locale: &str) -> JsResult<Self> {
        Self::build(locale, &JsValue::Undefined)
    }

    pub fn with_locales(locales: &JsArray<String>) -> JsResult<Self> {
        Self::with_locale(&first_locale(locales)?)
    }

    pub fn with_locale_options(locale: &str, options: &JsValue) -> JsResult<Self> {
        Self::build(locale, options)
    }

    pub fn with_locales_options(locales: &JsArray<String>, options: &JsValue) -> JsResult<Self> {
        Self::with_locale_options(&first_locale(locales)?, options)
    }

    pub fn format(&self, value: f64) -> String {
        self.format_to_parts(value)
            .values()
            .into_iter()
            .flatten()
            .map(|part| part.value)
            .collect::<Vec<_>>()
            .concat()
    }

    pub fn format_to_parts(&self, value: f64) -> JsArray<IntlNumberFormatPart> {
        if value.is_nan() {
            return JsArray::from_dense(vec![IntlNumberFormatPart::new("nan", "NaN")]);
        }
        if value.is_infinite() {
            let mut parts = Vec::new();
            if value.is_sign_negative() {
                parts.push(IntlNumberFormatPart::new("minusSign", "-"));
            }
            parts.push(IntlNumberFormatPart::new("infinity", "∞"));
            return JsArray::from_dense(parts);
        }
        let scaled = if self.style == "percent" { value * 100.0 } else { value };
        let negative = scaled.is_sign_negative() && scaled != 0.0;
        let absolute = scaled.abs();
        let rendered = format!("{:.*}", usize::from(self.maximum_fraction_digits), absolute);
        let (integer, fraction) = rendered.split_once('.').unwrap_or((&rendered, ""));
        let integer = integer_with_minimum(integer, usize::from(self.minimum_integer_digits));
        let groups = if self.use_grouping { group_integer(&integer) } else { vec![integer] };
        let mut parts = Vec::new();
        if negative {
            parts.push(IntlNumberFormatPart::new("minusSign", "-"));
        }
        if self.style == "currency" {
            parts.push(IntlNumberFormatPart::new(
                "currency",
                self.currency_label.as_deref().unwrap_or_default(),
            ));
            if self.currency_display == "code" || self.currency_display == "name" {
                parts.push(IntlNumberFormatPart::new("literal", " "));
            }
        }
        for (index, group) in groups.into_iter().enumerate() {
            if index > 0 {
                parts.push(IntlNumberFormatPart::new("group", ","));
            }
            parts.push(IntlNumberFormatPart::new("integer", group));
        }
        let minimum = usize::from(self.minimum_fraction_digits);
        let trimmed = fraction.trim_end_matches('0');
        let kept = trimmed.len().max(minimum).min(fraction.len());
        if kept > 0 {
            parts.push(IntlNumberFormatPart::new("decimal", "."));
            parts.push(IntlNumberFormatPart::new("fraction", &fraction[..kept]));
        }
        if self.style == "percent" {
            parts.push(IntlNumberFormatPart::new("percentSign", "%"));
        }
        JsArray::from_dense(parts)
    }

    pub fn resolved_options(&self) -> IntlResolvedNumberFormatOptions {
        IntlResolvedNumberFormatOptions {
            locale: self.locale.clone(),
            style: self.style.clone(),
            minimum_integer_digits: self.minimum_integer_digits,
            minimum_fraction_digits: self.minimum_fraction_digits,
            maximum_fraction_digits: self.maximum_fraction_digits,
            use_grouping: self.use_grouping,
        }
    }

    fn build(locale: &str, options: &JsValue) -> JsResult<Self> {
        let locale = canonical_locale(locale)?;
        validate_locale_matcher(options)?;
        let style = enum_option(options, "style", &["decimal", "percent", "currency"])?
            .unwrap_or_else(|| "decimal".to_owned());
        let currency = string_option(options, "currency")?;
        let currency_display = enum_option(
            options,
            "currencyDisplay",
            &["symbol", "narrowSymbol", "code", "name"],
        )?.unwrap_or_else(|| "symbol".to_owned());
        if style == "currency" && currency.as_deref().is_none_or(str::is_empty) {
            return Err(type_error("Intl.NumberFormat currency style requires a currency code"));
        }
        let currency_label = currency
            .as_deref()
            .map(|value| currency_value(value, &currency_display))
            .transpose()?;
        let minimum_integer_digits = integer_option(options, "minimumIntegerDigits", 1, 21)?.unwrap_or(1);
        let minimum_fraction_digits = integer_option(options, "minimumFractionDigits", 0, 20)?.unwrap_or(0);
        let default_maximum = if style == "currency" { 2 } else { 3 };
        let maximum_fraction_digits = integer_option(
            options,
            "maximumFractionDigits",
            minimum_fraction_digits,
            20,
        )?
        .unwrap_or(default_maximum.max(minimum_fraction_digits));
        let use_grouping = boolean_option(options, "useGrouping")?.unwrap_or(true);
        Ok(Self {
            identity: ObjectIdentity::new(),
            locale,
            style,
            currency_label,
            currency_display,
            use_grouping,
            minimum_integer_digits,
            minimum_fraction_digits,
            maximum_fraction_digits,
        })
    }
}

impl Default for IntlNumberFormat {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectIdentityCarrier for IntlNumberFormat {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

#[derive(Clone, Debug)]
pub struct IntlCollator {
    identity: ObjectIdentity,
    options: IntlResolvedCollatorOptions,
}

impl IntlCollator {
    pub fn new() -> Self {
        Self::build(DEFAULT_LOCALE, &JsValue::Undefined).expect("default Intl.Collator")
    }

    pub fn with_locale(locale: &str) -> JsResult<Self> {
        Self::build(locale, &JsValue::Undefined)
    }

    pub fn with_locales(locales: &JsArray<String>) -> JsResult<Self> {
        Self::with_locale(&first_locale(locales)?)
    }

    pub fn with_locale_options(locale: &str, options: &JsValue) -> JsResult<Self> {
        Self::build(locale, options)
    }

    pub fn with_locales_options(locales: &JsArray<String>, options: &JsValue) -> JsResult<Self> {
        Self::with_locale_options(&first_locale(locales)?, options)
    }

    pub fn compare(&self, left: &str, right: &str) -> f64 {
        let normalize = |value: &str| {
            let filtered = if self.options.ignore_punctuation {
                value.chars().filter(|character| character.is_alphanumeric() || character.is_whitespace()).collect()
            } else {
                value.to_owned()
            };
            match self.options.sensitivity.as_str() {
                "base" | "accent" => filtered.to_lowercase(),
                _ => filtered,
            }
        };
        let left_key = normalize(left);
        let right_key = normalize(right);
        let mut ordering = if self.options.numeric {
            numeric_string_compare(&left_key, &right_key)
        } else {
            left_key.cmp(&right_key)
        };
        if ordering == std::cmp::Ordering::Equal &&
            self.options.case_first != "false" &&
            left != right
        {
            ordering = if self.options.case_first == "upper" {
                left.cmp(right)
            } else {
                right.cmp(left)
            };
        }
        match ordering {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Equal => 0.0,
            std::cmp::Ordering::Greater => 1.0,
        }
    }

    pub fn resolved_options(&self) -> IntlResolvedCollatorOptions {
        self.options.clone()
    }

    fn build(locale: &str, options: &JsValue) -> JsResult<Self> {
        let locale = canonical_locale(locale)?;
        validate_locale_matcher(options)?;
        let usage = enum_option(options, "usage", &["sort", "search"])?
            .unwrap_or_else(|| "sort".to_owned());
        let sensitivity = enum_option(options, "sensitivity", &["base", "accent", "case", "variant"])?
            .unwrap_or_else(|| "variant".to_owned());
        let case_first = enum_option(options, "caseFirst", &["upper", "lower", "false"])?
            .unwrap_or_else(|| "false".to_owned());
        Ok(Self {
            identity: ObjectIdentity::new(),
            options: IntlResolvedCollatorOptions {
                locale,
                usage,
                sensitivity,
                ignore_punctuation: boolean_option(options, "ignorePunctuation")?.unwrap_or(false),
                numeric: boolean_option(options, "numeric")?.unwrap_or(false),
                case_first,
            },
        })
    }
}

impl Default for IntlCollator {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectIdentityCarrier for IntlCollator {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

fn canonical_locale(locale: &str) -> JsResult<String> {
    match locale {
        "en" | "en-US" => Ok(DEFAULT_LOCALE.to_owned()),
        _ => Err(range_error(format!(
            "Intl locale '{locale}' is outside the deterministic locale set"
        ))),
    }
}

fn validate_locale_matcher(options: &JsValue) -> JsResult<()> {
    enum_option(options, "localeMatcher", &["lookup", "best fit"]).map(|_| ())
}

fn first_locale(locales: &JsArray<String>) -> JsResult<String> {
    locales
        .values()
        .first()
        .and_then(|value| value.clone())
        .ok_or_else(|| range_error("Intl locale list must contain at least one locale"))
}

fn options_object(options: &JsValue) -> JsResult<Option<std::cell::Ref<'_, crate::JsObject>>> {
    match options {
        JsValue::Undefined | JsValue::Null => Ok(None),
        JsValue::Object(object) => object
            .try_borrow()
            .map(Some)
            .map_err(|_| type_error("Intl options object is already mutably borrowed")),
        _ => Err(type_error("Intl options must be an object")),
    }
}

fn option_value(options: &JsValue, name: &str) -> JsResult<JsValue> {
    Ok(options_object(options)?
        .map_or(JsValue::Undefined, |object| object.get(name)))
}

fn string_option(options: &JsValue, name: &str) -> JsResult<Option<String>> {
    match option_value(options, name)? {
        JsValue::Undefined => Ok(None),
        JsValue::String(value) => value
            .to_utf8()
            .map(Some)
            .map_err(|_| type_error(format!("Intl option '{name}' is not a native string"))),
        _ => Err(type_error(format!("Intl option '{name}' must be a string"))),
    }
}

fn enum_option(options: &JsValue, name: &str, accepted: &[&str]) -> JsResult<Option<String>> {
    let value = string_option(options, name)?;
    if value.as_deref().is_some_and(|candidate| !accepted.contains(&candidate)) {
        return Err(range_error(format!("Intl option '{name}' has an unsupported value")));
    }
    Ok(value)
}

fn boolean_option(options: &JsValue, name: &str) -> JsResult<Option<bool>> {
    match option_value(options, name)? {
        JsValue::Undefined => Ok(None),
        JsValue::Bool(value) => Ok(Some(value)),
        _ => Err(type_error(format!("Intl option '{name}' must be boolean"))),
    }
}

fn integer_option(options: &JsValue, name: &str, minimum: u8, maximum: u8) -> JsResult<Option<u8>> {
    match option_value(options, name)? {
        JsValue::Undefined => Ok(None),
        JsValue::Number(value)
            if value.is_finite() && value.fract() == 0.0 &&
                value >= f64::from(minimum) && value <= f64::from(maximum) => Ok(Some(value as u8)),
        _ => Err(range_error(format!("Intl option '{name}' is outside its supported range"))),
    }
}

fn append_separated(
    target: &mut Vec<IntlDateTimeFormatPart>,
    values: Vec<IntlDateTimeFormatPart>,
    separator: &str,
) {
    for (index, value) in values.into_iter().enumerate() {
        if index > 0 {
            target.push(IntlDateTimeFormatPart::new("literal", separator));
        }
        target.push(value);
    }
}

#[derive(Clone, Copy)]
struct UtcComponents {
    year: i64,
    month: u32,
    day: u32,
    weekday: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

fn utc_components(milliseconds: f64) -> UtcComponents {
    let seconds = (milliseconds / 1000.0).floor() as i64;
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400) as u32;
    let (year, month, day) = civil_from_days(days);
    UtcComponents {
        year,
        month,
        day,
        weekday: (days + 4).rem_euclid(7) as u32,
        hour: seconds_of_day / 3_600,
        minute: (seconds_of_day % 3_600) / 60,
        second: seconds_of_day % 60,
    }
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month as u32, day as u32)
}

fn number_value(value: impl std::fmt::Display, style: &str) -> String {
    let rendered = value.to_string();
    if style == "2-digit" && rendered.len() < 2 {
        format!("0{rendered}")
    } else {
        rendered
    }
}

fn month_value(month: u32, style: &str) -> String {
    const LONG: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const SHORT: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    match style {
        "long" => LONG[(month - 1) as usize].to_owned(),
        "short" => SHORT[(month - 1) as usize].to_owned(),
        "narrow" => LONG[(month - 1) as usize][..1].to_owned(),
        _ => number_value(month, style),
    }
}

fn weekday_name(weekday: u32, style: &str) -> String {
    const LONG: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    const SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    match style {
        "long" => LONG[weekday as usize].to_owned(),
        "short" => SHORT[weekday as usize].to_owned(),
        _ => LONG[weekday as usize][..1].to_owned(),
    }
}

fn era_value(style: &str) -> String {
    match style {
        "long" => "Anno Domini",
        "narrow" => "A",
        _ => "AD",
    }
    .to_owned()
}

fn currency_value(currency: &str, display: &str) -> JsResult<String> {
    let currency = currency.to_ascii_uppercase();
    let value = match display {
        "code" => currency.clone(),
        "name" => match currency.as_str() {
            "USD" => "US dollars".to_owned(),
            "EUR" => "euros".to_owned(),
            "GBP" => "British pounds".to_owned(),
            "JPY" => "Japanese yen".to_owned(),
            _ => {
                return Err(range_error(format!(
                    "Intl currency name data is unavailable for '{currency}'"
                )));
            }
        },
        _ => match currency.as_str() {
            "USD" => "$".to_owned(),
            "EUR" => "€".to_owned(),
            "GBP" => "£".to_owned(),
            "JPY" => "¥".to_owned(),
            "CNY" => "CN¥".to_owned(),
            "INR" => "₹".to_owned(),
            _ => currency,
        },
    };
    Ok(value)
}

fn integer_with_minimum(value: &str, minimum: usize) -> String {
    if value.len() >= minimum {
        value.to_owned()
    } else {
        format!("{}{}", "0".repeat(minimum - value.len()), value)
    }
}

fn group_integer(value: &str) -> Vec<String> {
    let first = value.len() % 3;
    let mut groups = Vec::new();
    let mut index = 0;
    if first > 0 {
        groups.push(value[..first].to_owned());
        index = first;
    }
    while index < value.len() {
        groups.push(value[index..index + 3].to_owned());
        index += 3;
    }
    groups
}

fn numeric_string_compare(left: &str, right: &str) -> std::cmp::Ordering {
    match (left.parse::<u128>(), right.parse::<u128>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}
