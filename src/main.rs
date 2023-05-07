use std::str::FromStr;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use libastro_sys::{cal_mjd, utc_gst};
use once_cell::sync::Lazy;
use tzf_rs::DefaultFinder;

fn utc_to_float(time: NaiveTime) -> f64 {
	(time.hour() as f64)
		+ (time.minute() as f64 / (60.0))
		+ (time.second() as f64 / (60.0 * 60.0))
		+ (time.nanosecond() as f64 / (60.0 * 60.0 * 1_000_000_000.0))
}

pub fn mjd_from_gregorian_date(date: NaiveDate) -> f64 {
	let dy = date.day() as f64;
	let mn = date.month() as i32;
	let yr = date.year() as i32;
	let mut mjd = 0.0;
	unsafe { cal_mjd(mn, dy, yr, &mut mjd as *mut f64) };
	mjd
}

pub fn mjd_from_gregorian_datetime(datetime: NaiveDateTime) -> f64 {
	let mjd = mjd_from_gregorian_date(datetime.date());
	mjd + utc_to_float(datetime.time())
}

pub fn greenwich_mean_sidereal_time(datetime: NaiveDateTime) -> f64 {
	let mut gst = 0.0;
	let utc = utc_to_float(datetime.time());
	let mjd = mjd_from_gregorian_date(datetime.date()).floor();
	unsafe { utc_gst(mjd, utc, &mut gst as *mut f64) };
	gst
}

/// Find the timezone for the given coordinates
fn get_timezone(latitude: f64, longitude: f64) -> Result<Tz> {
	let finder = DefaultFinder::new();
	let timezone = finder.get_tz_names(longitude, latitude);
	let tz_str = match timezone.len() {
		0 => Err(anyhow!("No timezones found")),
		1 => Ok(timezone.first().expect("already checked").to_owned()),
		_ => Err(anyhow!("Todo: Allow picking a timezone name")),
	}?;
	Tz::from_str(tz_str).map_err(|e| anyhow!("Could not convert string: {e}"))
}

fn decimal_to_time(dec_time: f64) -> Result<NaiveTime> {
	let hr = dec_time;
	let min = hr.fract() * 60.0;
	let sec = min.fract() * 60.0;
	let ns = sec.fract() * 1_000_000_000.0;

	NaiveTime::from_hms_nano_opt(hr as u32, min as u32, sec as u32, ns as u32)
		.ok_or(anyhow!("Time conversion failed, time: {dec_time}"))
}

fn local_mean_sidereal_time(gmst: f64, longitude: f64) -> f64 {
	24.0 * ((gmst + longitude / 15.0) / 24.0).fract()
}

const SPOTISWOODE_PEAK_TIME: Lazy<NaiveTime> =
	Lazy::new(|| NaiveTime::from_hms_opt(13, 30, 0).unwrap());

fn info_text(utc_datetime: DateTime<Utc>, latitude: f64, longitude: f64) -> Result<String> {
	const TIME_FMT_STRING: &str = "%T.%6f";
	const TIME_ZONE_FMT_STRING: &str = "%T.%6f %z/%Z";

	let mut info = String::new();
	// don't use more than two digits of precision for coordinates
	let timezone: Tz = get_timezone(latitude, longitude).expect("No timezone");

	info.push_str(&format!(
		"           Zone for {:>5.1}, {:>5.1}: {:?}\n",
		latitude, longitude, timezone
	));

	let curr_date = utc_datetime.date_naive();

	info.push_str(&format!(
		"                  Gregorian Date: {}\n",
		curr_date
	));

	info.push_str(&format!(
		"                  Universal Time: {}\n",
		utc_datetime.format(TIME_ZONE_FMT_STRING)
	));

	let local_time = utc_datetime.with_timezone(&timezone);
	info.push_str(&format!(
		"                      Local Time: {}\n",
		local_time.format(TIME_ZONE_FMT_STRING)
	));

	let mjd = mjd_from_gregorian_datetime(utc_datetime.naive_utc());
	info.push_str(&format!("             Modified Julian Day: {}\n", mjd));

	let gmst = greenwich_mean_sidereal_time(utc_datetime.naive_utc());
	info.push_str(&format!(
		"    Greenwich mean Sidereal Time: {} \n",
		decimal_to_time(gmst)?.format(TIME_FMT_STRING)
	));

	let lmst = local_mean_sidereal_time(gmst, longitude);
	info.push_str(&format!(
		"        Local mean Sidereal Time: {}\n",
		decimal_to_time(lmst)?.format(TIME_FMT_STRING)
	));

	let time_until_peak = {
		let duration = SPOTISWOODE_PEAK_TIME.signed_duration_since(decimal_to_time(lmst)?);
		if duration.lt(&Duration::zero()) {
			// If the duration is negative, add 24 hours to it to get the time until the next occurrence.
			duration + chrono::Duration::hours(24)
		} else {
			duration
		}
	};
	info.push_str(&format!(
		"Time Until Spotiswoode Peak Time: {}\n",
		decimal_to_time(
			time_until_peak.num_nanoseconds().unwrap() as f64 / 1_000_000_000.0 / 60.0 / 60.0
		)?
		.format(TIME_FMT_STRING),
	));

	Ok(info)
}

fn main() -> Result<()> {
	let term = console::Term::buffered_stdout();

	const LATITUDE: f64 = 36.755;
	const LONGITUDE: f64 = 127.869;

	loop {
		let info = info_text(Utc::now(), LATITUDE, LONGITUDE)?;
		term.write_line(&info)?;
		term.flush()?;
		term.clear_last_lines(9)?;
	}
}
