//! Deterministic, closed internationalization carriers.

use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier, Undefined};

use crate::array::JsArray;
use crate::date::JsDate;
use crate::errors::{range_error, type_error, JsResult};
use crate::value::JsValue;
mod number_precision;
use number_precision::NumberPrecision;
mod number_input;
pub use number_input::IntlNumberInput;
mod grouping;
pub use grouping::IntlGrouping;

const DEFAULT_LOCALE: &str = "en-US";
const DEFAULT_TIME_ZONE: &str = "UTC";

pub fn integer_to_locale_string<Value: IntlNumberInput>(value: Value) -> String {
    IntlNumberFormat::new().format(value)
}

pub fn integer_to_locale_string_with_undefined<Value: IntlNumberInput>(
    value: Value,
    _locale: Undefined,
) -> String {
    integer_to_locale_string(value)
}

pub fn integer_to_locale_string_with_undefined_options<Value: IntlNumberInput>(
    value: Value,
    _locale: Undefined,
    options: &JsValue,
) -> JsResult<String> {
    Ok(IntlNumberFormat::build(DEFAULT_LOCALE, options)?.format(value))
}

pub fn integer_to_locale_string_with_locale<Value: IntlNumberInput>(
    value: Value,
    locale: &str,
) -> JsResult<String> {
    Ok(IntlNumberFormat::with_locale(locale)?.format(value))
}

pub fn integer_to_locale_string_with_options<Value: IntlNumberInput>(
    value: Value,
    locale: &str,
    options: &JsValue,
) -> JsResult<String> {
    Ok(IntlNumberFormat::with_locale_options(locale, options)?.format(value))
}

pub fn integer_to_locale_string_with_locales<Value: IntlNumberInput>(
    value: Value,
    locales: &JsArray<String>,
) -> JsResult<String> {
    Ok(IntlNumberFormat::with_locales(locales)?.format(value))
}

pub fn integer_to_locale_string_with_locales_options<Value: IntlNumberInput>(
    value: Value,
    locales: &JsArray<String>,
    options: &JsValue,
) -> JsResult<String> {
    Ok(IntlNumberFormat::with_locales_options(locales, options)?.format(value))
}

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
    precision: NumberPrecision,
    use_grouping: Option<String>,
    currency: Option<String>,
    currency_display: Option<String>,
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
        f64::from(self.precision.minimum_integer)
    }

    pub fn minimum_fraction_digits(&self) -> Option<f64> {
        self.precision.minimum_fraction.map(f64::from)
    }

    pub fn maximum_fraction_digits(&self) -> Option<f64> {
        self.precision.maximum_fraction.map(f64::from)
    }

    pub fn use_grouping(&self) -> IntlGrouping {
        match &self.use_grouping {
            Some(strategy) => IntlGrouping::Strategy(strategy.clone()),
            None => IntlGrouping::Disabled,
        }
    }
    pub fn minimum_significant_digits(&self) -> Option<f64> {
        self.precision.minimum_significant.map(f64::from)
    }
    pub fn maximum_significant_digits(&self) -> Option<f64> {
        self.precision.maximum_significant.map(f64::from)
    }
    pub fn currency(&self) -> Option<String> {
        self.currency.clone()
    }
    pub fn currency_display(&self) -> Option<String> {
        self.currency_display.clone()
    }
    pub fn currency_sign(&self) -> Option<String> {
        self.currency.as_ref().map(|_| "standard".to_owned())
    }
    pub fn unit(&self) -> Option<String> {
        None
    }
    pub fn unit_display(&self) -> Option<String> {
        None
    }
    pub fn notation(&self) -> String {
        "standard".to_owned()
    }
    pub fn compact_display(&self) -> Option<String> {
        None
    }
    pub fn sign_display(&self) -> String {
        "auto".to_owned()
    }
    pub fn rounding_priority(&self) -> String {
        "auto".to_owned()
    }
    pub fn rounding_increment(&self) -> f64 {
        1.0
    }
    pub fn rounding_mode(&self) -> String {
        "halfExpand".to_owned()
    }
    pub fn trailing_zero_display(&self) -> String {
        "auto".to_owned()
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
        let time_zone =
            string_option(options, "timeZone")?.unwrap_or_else(|| DEFAULT_TIME_ZONE.to_owned());
        if time_zone != DEFAULT_TIME_ZONE {
            return Err(range_error(format!(
                "Intl.DateTimeFormat supports only the deterministic '{DEFAULT_TIME_ZONE}' time zone"
            )));
        }
        let mut fields = DateTimeFields {
            weekday: enum_option(options, "weekday", &["long", "short", "narrow"])?,
            era: enum_option(options, "era", &["long", "short", "narrow"])?,
            year: enum_option(options, "year", &["numeric", "2-digit"])?,
            month: enum_option(
                options,
                "month",
                &["numeric", "2-digit", "long", "short", "narrow"],
            )?,
            day: enum_option(options, "day", &["numeric", "2-digit"])?,
            hour: enum_option(options, "hour", &["numeric", "2-digit"])?,
            minute: enum_option(options, "minute", &["numeric", "2-digit"])?,
            second: enum_option(options, "second", &["numeric", "2-digit"])?,
            time_zone_name: enum_option(options, "timeZoneName", &["long", "short"])?,
            hour12: boolean_option(options, "hour12")?.unwrap_or(true),
        };
        if fields.weekday.is_none()
            && fields.era.is_none()
            && fields.year.is_none()
            && fields.month.is_none()
            && fields.day.is_none()
            && fields.hour.is_none()
            && fields.minute.is_none()
            && fields.second.is_none()
            && fields.time_zone_name.is_none()
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
            return JsArray::from_dense(vec![IntlDateTimeFormatPart::new(
                "literal",
                "Invalid Date",
            )]);
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
                if style == "long" {
                    "Coordinated Universal Time"
                } else {
                    "UTC"
                },
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
    use_grouping: Option<String>,
    precision: NumberPrecision,
    currency: Option<String>,
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

    pub fn format<Value: IntlNumberInput>(&self, value: Value) -> String {
        self.format_to_parts(value)
            .values()
            .into_iter()
            .flatten()
            .map(|part| part.value)
            .collect::<Vec<_>>()
            .concat()
    }

    pub fn format_to_parts<Value: IntlNumberInput>(
        &self,
        value: Value,
    ) -> JsArray<IntlNumberFormatPart> {
        let (text, negative_zero) = value.into_intl_decimal();
        if text == "NaN" {
            return JsArray::from_dense(vec![IntlNumberFormatPart::new("nan", "NaN")]);
        }
        if text == "Infinity" || text == "-Infinity" {
            let mut parts = Vec::new();
            if text.starts_with('-') {
                parts.push(IntlNumberFormatPart::new("minusSign", "-"));
            }
            parts.push(IntlNumberFormatPart::new("infinity", "∞"));
            return JsArray::from_dense(parts);
        }
        let negative = text.starts_with('-') || negative_zero;
        let (integer, fraction) = self
            .precision
            .format(text.trim_start_matches('-'), self.style == "percent");
        let groups = if self.use_grouping.is_some()
            && (self.use_grouping.as_deref() != Some("min2") || integer.len() > 4)
        {
            group_integer(&integer)
        } else {
            vec![integer]
        };
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
        if !fraction.is_empty() {
            parts.push(IntlNumberFormatPart::new("decimal", "."));
            parts.push(IntlNumberFormatPart::new("fraction", fraction));
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
            precision: self.precision.clone(),
            use_grouping: self.use_grouping.clone(),
            currency: if self.style == "currency" {
                self.currency.clone()
            } else {
                None
            },
            currency_display: if self.style == "currency" {
                Some(self.currency_display.clone())
            } else {
                None
            },
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
        )?
        .unwrap_or_else(|| "symbol".to_owned());
        if style == "currency" && currency.as_deref().is_none_or(str::is_empty) {
            return Err(type_error(
                "Intl.NumberFormat currency style requires a currency code",
            ));
        }
        let currency_label = currency
            .as_deref()
            .map(|value| currency_value(value, &currency_display))
            .transpose()?;
        for (name, values) in [
            ("numberingSystem", &["latn"][..]),
            ("currencySign", &["standard"][..]),
            ("notation", &["standard"][..]),
            ("compactDisplay", &["short", "long"][..]),
            ("unitDisplay", &["short", "long", "narrow"][..]),
            ("signDisplay", &["auto"][..]),
            ("roundingPriority", &["auto"][..]),
            ("roundingMode", &["halfExpand"][..]),
            ("trailingZeroDisplay", &["auto"][..]),
        ] {
            enum_option(options, name, values)?;
        }
        if string_option(options, "unit")?.is_some() {
            return Err(range_error(
                "Intl.NumberFormat unit options are not supported",
            ));
        }
        if integer_option(options, "roundingIncrement", 1, 5000)?.unwrap_or(1) != 1 {
            return Err(range_error(
                "Intl.NumberFormat supports only roundingIncrement 1",
            ));
        }
        let use_grouping = match option_value(options, "useGrouping")? {
            JsValue::Undefined => Some("auto".to_owned()),
            JsValue::Bool(false) => None,
            JsValue::Bool(true) => Some("always".to_owned()),
            JsValue::String(value) => match value
                .to_utf8()
                .map_err(|_| range_error("Invalid grouping strategy"))?
                .as_str()
            {
                "auto" => Some("auto".to_owned()),
                "always" => Some("always".to_owned()),
                "min2" => Some("min2".to_owned()),
                "true" | "false" => Some("auto".to_owned()),
                _ => return Err(range_error("Invalid grouping strategy")),
            },
            _ => return Err(range_error("Invalid grouping strategy")),
        };
        let currency_digits = if currency
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("JPY"))
        {
            0
        } else {
            2
        };
        let precision = NumberPrecision::new(
            options,
            if style == "currency" {
                currency_digits
            } else {
                0
            },
            if style == "currency" {
                currency_digits
            } else if style == "percent" {
                0
            } else {
                3
            },
        )?;
        Ok(Self {
            identity: ObjectIdentity::new(),
            locale,
            style,
            currency_label,
            currency_display,
            use_grouping,
            precision,
            currency: currency.map(|value| value.to_ascii_uppercase()),
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
                value
                    .chars()
                    .filter(|character| character.is_alphanumeric() || character.is_whitespace())
                    .collect()
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
        if ordering == std::cmp::Ordering::Equal
            && self.options.case_first != "false"
            && left != right
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
        let sensitivity = enum_option(
            options,
            "sensitivity",
            &["base", "accent", "case", "variant"],
        )?
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
    Ok(options_object(options)?.map_or(JsValue::Undefined, |object| object.get(name)))
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
    if value
        .as_deref()
        .is_some_and(|candidate| !accepted.contains(&candidate))
    {
        return Err(range_error(format!(
            "Intl option '{name}' has an unsupported value"
        )));
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

fn integer_option(
    options: &JsValue,
    name: &str,
    minimum: u16,
    maximum: u16,
) -> JsResult<Option<u16>> {
    match option_value(options, name)? {
        JsValue::Undefined => Ok(None),
        JsValue::Number(value)
            if value.is_finite() && value >= f64::from(minimum) && value <= f64::from(maximum) =>
        {
            Ok(Some(value as u16))
        }
        _ => Err(range_error(format!(
            "Intl option '{name}' is outside its supported range"
        ))),
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
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
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
    const LONG: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const SHORT: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    match style {
        "long" => LONG[(month - 1) as usize].to_owned(),
        "short" => SHORT[(month - 1) as usize].to_owned(),
        "narrow" => LONG[(month - 1) as usize][..1].to_owned(),
        _ => number_value(month, style),
    }
}

fn weekday_name(weekday: u32, style: &str) -> String {
    const LONG: [&str; 7] = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
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
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut left_index = 0;
    let mut right_index = 0;

    while left_index < left.len() && right_index < right.len() {
        if left[left_index].is_ascii_digit() && right[right_index].is_ascii_digit() {
            let left_end = digit_run_end(left, left_index);
            let right_end = digit_run_end(right, right_index);
            let left_significant = trim_numeric_leading_zeroes(&left[left_index..left_end]);
            let right_significant = trim_numeric_leading_zeroes(&right[right_index..right_end]);
            let ordering = left_significant
                .len()
                .cmp(&right_significant.len())
                .then_with(|| left_significant.cmp(right_significant));
            if ordering != std::cmp::Ordering::Equal {
                return ordering;
            }
            left_index = left_end;
            right_index = right_end;
            continue;
        }

        let ordering = left[left_index].cmp(&right[right_index]);
        if ordering != std::cmp::Ordering::Equal {
            return ordering;
        }
        left_index += 1;
        right_index += 1;
    }

    (left.len() - left_index).cmp(&(right.len() - right_index))
}

fn digit_run_end(value: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < value.len() && value[end].is_ascii_digit() {
        end += 1;
    }
    end
}

fn trim_numeric_leading_zeroes(value: &[u8]) -> &[u8] {
    let first_significant = value
        .iter()
        .position(|digit| *digit != b'0')
        .unwrap_or(value.len());
    &value[first_significant..]
}
