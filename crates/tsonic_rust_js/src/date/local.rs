use super::{civil_from_days, JsDate, MS_PER_DAY};
use crate::errors::{range_error, JsResult};

fn offset_at(seconds: i64) -> JsResult<i32> {
    let requested = std::env::var_os("TZ");
    let offset = match requested {
        Some(requested) if requested.is_empty() => return Ok(0),
        Some(requested) => {
            let requested = requested
                .to_str()
                .ok_or_else(|| range_error("TZ is not a valid timezone identifier"))?;
            if let Some(zone) = tzdb::tz_by_name(requested) {
                zone.find_local_time_type(seconds)
                    .map(|local| local.ut_offset())
            } else {
                let zone = tz::TimeZone::from_posix_tz(requested)
                    .map_err(|error| range_error(format!("Cannot load timezone: {error}")))?;
                zone.find_local_time_type(seconds)
                    .map(|local| local.ut_offset())
            }
        }
        None => tzdb::local_tz()
            .ok_or_else(|| range_error("Cannot determine the system timezone"))?
            .find_local_time_type(seconds)
            .map(|local| local.ut_offset()),
    };
    offset.map_err(|error| range_error(format!("Cannot resolve local time: {error}")))
}

impl JsDate {
    fn local_component(&self, index: usize) -> JsResult<f64> {
        let millis = self.get_time();
        if !millis.is_finite() {
            return Ok(f64::NAN);
        }
        let millis = millis as i64;
        let offset = offset_at(millis.div_euclid(1_000))?;
        let local_millis = millis + i64::from(offset) * 1_000;
        let days = local_millis.div_euclid(MS_PER_DAY);
        let remainder = local_millis.rem_euclid(MS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        Ok([
            f64::from(year),
            f64::from(month - 1),
            f64::from(day),
            (days + 4).rem_euclid(7) as f64,
            (remainder / 3_600_000) as f64,
            (remainder / 60_000 % 60) as f64,
            (remainder / 1_000 % 60) as f64,
            (remainder % 1_000) as f64,
            f64::from(-(offset / 60)),
        ][index])
    }

    pub fn get_full_year(&self) -> JsResult<f64> {
        self.local_component(0)
    }

    pub fn get_month(&self) -> JsResult<f64> {
        self.local_component(1)
    }

    pub fn get_date(&self) -> JsResult<f64> {
        self.local_component(2)
    }

    pub fn get_day(&self) -> JsResult<f64> {
        self.local_component(3)
    }

    pub fn get_hours(&self) -> JsResult<f64> {
        self.local_component(4)
    }

    pub fn get_minutes(&self) -> JsResult<f64> {
        self.local_component(5)
    }

    pub fn get_seconds(&self) -> JsResult<f64> {
        self.local_component(6)
    }

    pub fn get_milliseconds(&self) -> f64 {
        self.get_utc_milliseconds_number()
    }

    pub fn get_timezone_offset(&self) -> JsResult<f64> {
        self.local_component(8)
    }
}
