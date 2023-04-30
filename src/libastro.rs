use chrono::{Datelike, NaiveDateTime, Timelike};

/// Given a date in months, mn, days, dy, years, yr,
/// return the modified Julian date (number of days elapsed since 1900 jan 0.5),
pub fn mjd_from_gregorian(datetime: NaiveDateTime) -> f64 {
	let day_fraction = {
		let time = datetime.time();
		let day = datetime.day();
		let day_frac = day as f64
			+ (time.hour() as f64 / 24.0)
			+ (time.minute() as f64 / 60.0)
			+ (time.second() as f64 / 3600.0);
		dbg!(&day_frac);
		day_frac
	};
	let mn = datetime.month() as i32;
	let yr = datetime.year() as i32;
	let mut mjd = 0.0;
	unsafe { libastro_sys::cal_mjd(mn, day_fraction, yr, &mut mjd as *mut f64) };
	mjd
}
