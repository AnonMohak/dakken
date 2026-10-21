//! The local hour, for the night owl badge. The standard library knows only
//! UTC, so this asks the system: `GetLocalTime` on Windows, `localtime_r`
//! elsewhere. Both are in libraries every program already links.

/// The hour of the local time now, 0 to 23.
#[cfg(windows)]
pub fn hour() -> Option<u32> {
    #[repr(C)]
    #[derive(Default)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }
    unsafe extern "system" {
        fn GetLocalTime(time: *mut SystemTime);
    }
    let mut time = SystemTime::default();
    // SAFETY: GetLocalTime fills the struct it gets, which has the
    // SYSTEMTIME layout.
    unsafe { GetLocalTime(&mut time) };
    (time.hour < 24).then_some(u32::from(time.hour))
}

/// The hour of the local time now, 0 to 23.
#[cfg(unix)]
pub fn hour() -> Option<u32> {
    // The start of `struct tm` is the same on Linux (glibc and musl) and
    // macOS; the rest only needs room.
    #[repr(C)]
    struct Tm {
        sec: i32,
        min: i32,
        hour: i32,
        rest: [u64; 8],
    }
    unsafe extern "C" {
        fn time(out: *mut i64) -> i64;
        fn localtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
    }
    let mut tm = Tm {
        sec: 0,
        min: 0,
        hour: -1,
        rest: [0; 8],
    };
    // SAFETY: time_t is 64 bits on the 64-bit targets dakken builds for,
    // and `Tm` is at least as big as `struct tm`.
    let ok = unsafe {
        let now = time(std::ptr::null_mut());
        !localtime_r(&now, &mut tm).is_null()
    };
    (ok && (0..24).contains(&tm.hour)).then_some(tm.hour as u32)
}

#[cfg(not(any(windows, unix)))]
pub fn hour() -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_local_hour_is_a_clock_hour() {
        let hour = super::hour().expect("the system gives a local time");
        assert!(hour < 24);
    }
}
