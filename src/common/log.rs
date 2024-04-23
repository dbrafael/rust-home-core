pub const COLOR_DEFAULT: &str = "\x1b[0m";
pub const LOG_COLOR: &str = "\x1b[0m";
pub const WARN_COLOR: &str = "\x1b[33m";
pub const ERROR_COLOR: &str = "\x1b[31m";

pub enum LogLevel {
    Log,
    Warn,
    Error,
}

pub fn log_message(level: LogLevel, message: &str) -> String {
    let color = match level {
        LogLevel::Log => LOG_COLOR,
        LogLevel::Warn => WARN_COLOR,
        LogLevel::Error => ERROR_COLOR,
    };
    let prefix = match level {
        LogLevel::Log => "LOG",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    };
    let message = format!("[{}] {}{}{}", prefix, color, message, COLOR_DEFAULT);
    println!("{}", message);
    message
}
