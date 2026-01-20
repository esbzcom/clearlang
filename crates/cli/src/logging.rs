use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Off,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn from_verbosity(verbosity: u8) -> Self {
        match verbosity {
            0 => LogLevel::Off,
            1 => LogLevel::Info,
            2 => LogLevel::Debug,
            _ => LogLevel::Trace,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Off => "off",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Logger {
    level: LogLevel,
}

impl Logger {
    pub fn from_env(verbosity: u8) -> Self {
        let base = LogLevel::from_verbosity(verbosity);
        let level = std::env::var("CLG_LOG")
            .ok()
            .and_then(|value| parse_log_level(&value))
            .unwrap_or(base);
        Logger { level }
    }

    pub fn enabled(self, level: LogLevel) -> bool {
        self.level >= level && self.level != LogLevel::Off
    }

    pub fn stage_start(self, name: &str) {
        if !self.enabled(LogLevel::Info) {
            return;
        }
        self.log(LogLevel::Info, "start", name, &[]);
    }

    pub fn stage_finish(self, name: &str, duration: Duration) {
        if !self.enabled(LogLevel::Info) {
            return;
        }
        let duration_ms = duration.as_millis().to_string();
        self.log(LogLevel::Info, "finish", name, &[("duration_ms", duration_ms)]);
    }

    pub fn summary(self, timings: &StageTimings) {
        if !self.enabled(LogLevel::Info) {
            return;
        }
        let mut total_ms: u128 = 0;
        let mut parts = Vec::with_capacity(timings.entries.len());
        for entry in &timings.entries {
            let ms = entry.duration.as_millis();
            total_ms += ms;
            parts.push(format!("{}={}ms", entry.name, ms));
        }
        let stages = parts.join(" ");
        self.log(
            LogLevel::Info,
            "summary",
            "pipeline",
            &[
                ("total_ms", total_ms.to_string()),
                ("stages", stages),
            ],
        );
    }

    pub fn event(self, level: LogLevel, event: &str, stage: &str, fields: &[(&str, String)]) {
        if !self.enabled(level) {
            return;
        }
        self.log(level, event, stage, fields);
    }

    fn log(self, level: LogLevel, event: &str, stage: &str, fields: &[(&str, String)]) {
        let mut line = String::new();
        line.push_str("clg");
        line.push_str(" level=");
        line.push_str(level.as_str());
        line.push_str(" event=");
        line.push_str(event);
        line.push_str(" stage=");
        line.push_str(stage);
        for (key, value) in fields {
            line.push(' ');
            line.push_str(key);
            line.push('=');
            line.push_str(&quote_value(value));
        }
        eprintln!("{line}");
    }
}

pub struct StageTimings {
    entries: Vec<StageTiming>,
}

impl StageTimings {
    pub fn new() -> Self {
        StageTimings { entries: Vec::new() }
    }

    pub fn start<'a>(&'a mut self, logger: Logger, name: &'static str) -> StageGuard<'a> {
        logger.stage_start(name);
        StageGuard {
            logger,
            name,
            start: Instant::now(),
            timings: self,
        }
    }
}

struct StageTiming {
    name: &'static str,
    duration: Duration,
}

pub struct StageGuard<'a> {
    logger: Logger,
    name: &'static str,
    start: Instant,
    timings: &'a mut StageTimings,
}

impl<'a> Drop for StageGuard<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.timings.entries.push(StageTiming {
            name: self.name,
            duration,
        });
        self.logger.stage_finish(self.name, duration);
    }
}

fn parse_log_level(value: &str) -> Option<LogLevel> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" => None,
        "off" | "0" => Some(LogLevel::Off),
        "info" | "1" | "warn" | "error" => Some(LogLevel::Info),
        "debug" | "2" => Some(LogLevel::Debug),
        "trace" | "3" => Some(LogLevel::Trace),
        _ => None,
    }
}

fn quote_value(value: &str) -> String {
    if value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
    {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbosity_maps_to_levels() {
        assert_eq!(LogLevel::from_verbosity(0), LogLevel::Off);
        assert_eq!(LogLevel::from_verbosity(1), LogLevel::Info);
        assert_eq!(LogLevel::from_verbosity(2), LogLevel::Debug);
        assert_eq!(LogLevel::from_verbosity(3), LogLevel::Trace);
    }

    #[test]
    fn parse_log_level_accepts_names_and_numbers() {
        assert_eq!(parse_log_level("off"), Some(LogLevel::Off));
        assert_eq!(parse_log_level("1"), Some(LogLevel::Info));
        assert_eq!(parse_log_level("debug"), Some(LogLevel::Debug));
        assert_eq!(parse_log_level("TRACE"), Some(LogLevel::Trace));
    }
}
