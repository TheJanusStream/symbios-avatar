#![allow(unsafe_code)]
//! LD_PRELOAD shim: every exported maths function returns the `libm`
//! crate's result. `SHIM_FN=<name|all>` with `SHIM_DIR=up|down` nudges that
//! function's result (both outputs of `sincosf`/`sincos`) `SHIM_ULPS` ulps
//! (default 1). `SHIM_LOG=1` prints each function's first call to stderr.
use std::sync::OnceLock;

struct Cfg {
    name: String,
    dir: i32,
    ulps: u32,
    log: bool,
}

fn cfg() -> &'static Cfg {
    static CFG: OnceLock<Cfg> = OnceLock::new();
    CFG.get_or_init(|| Cfg {
        name: std::env::var("SHIM_FN").unwrap_or_default(),
        dir: match std::env::var("SHIM_DIR").as_deref() {
            Ok("up") => 1,
            Ok("down") => -1,
            _ => 0,
        },
        ulps: std::env::var("SHIM_ULPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        log: std::env::var_os("SHIM_LOG").is_some(),
    })
}

fn seen(name: &'static str) {
    if !cfg().log {
        return;
    }
    static SEEN: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());
    let mut s = SEEN.lock().unwrap();
    if !s.contains(&name) {
        s.push(name);
        eprintln!("SHIM first call: {name}");
    }
}

fn hit(name: &str) -> i32 {
    let c = cfg();
    if c.dir != 0 && (c.name == name || c.name == "all") { c.dir } else { 0 }
}

fn nudge32(name: &'static str, v: f32) -> f32 {
    seen(name);
    let d = hit(name);
    if d == 0 || !v.is_finite() {
        return v;
    }
    let mut out = v;
    for _ in 0..cfg().ulps {
        out = libm::nextafterf(out, if d > 0 { f32::INFINITY } else { f32::NEG_INFINITY });
    }
    out
}

fn nudge64(name: &'static str, v: f64) -> f64 {
    seen(name);
    let d = hit(name);
    if d == 0 || !v.is_finite() {
        return v;
    }
    let mut out = v;
    for _ in 0..cfg().ulps {
        out = libm::nextafter(out, if d > 0 { f64::INFINITY } else { f64::NEG_INFINITY });
    }
    out
}

macro_rules! f32_1 { ($($n:ident),*) => { $(
    #[unsafe(no_mangle)] pub extern "C" fn $n(x: f32) -> f32 { nudge32(stringify!($n), libm::$n(x)) }
)* } }
macro_rules! f32_2 { ($($n:ident),*) => { $(
    #[unsafe(no_mangle)] pub extern "C" fn $n(x: f32, y: f32) -> f32 { nudge32(stringify!($n), libm::$n(x, y)) }
)* } }
macro_rules! f64_1 { ($($n:ident),*) => { $(
    #[unsafe(no_mangle)] pub extern "C" fn $n(x: f64) -> f64 { nudge64(stringify!($n), libm::$n(x)) }
)* } }
macro_rules! f64_2 { ($($n:ident),*) => { $(
    #[unsafe(no_mangle)] pub extern "C" fn $n(x: f64, y: f64) -> f64 { nudge64(stringify!($n), libm::$n(x, y)) }
)* } }

f32_1!(acosf, asinf, atanf, cosf, sinf, tanf, expf, logf, exp2f, log2f, log10f, tanhf);
f32_2!(atan2f, hypotf, powf);
f64_1!(acos, asin, atan, cos, sin, tan, exp, log, exp2, log2, log10);
f64_2!(atan2, hypot, pow);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sincosf(x: f32, s: *mut f32, c: *mut f32) {
    let (sv, cv) = libm::sincosf(x);
    unsafe {
        *s = nudge32("sincosf", sv);
        *c = nudge32("sincosf", cv);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sincos(x: f64, s: *mut f64, c: *mut f64) {
    let (sv, cv) = libm::sincos(x);
    unsafe {
        *s = nudge64("sincos", sv);
        *c = nudge64("sincos", cv);
    }
}
