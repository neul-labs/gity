use tracing::{error, warn};

/// Logs an error at WARN level and returns it for optional propagation.
/// Use this when an error should be logged but operation should continue.
#[macro_export]
macro_rules! log_warn {
    ($fmt:literal, $($arg:expr),*) => {
        |err| {
            warn!(concat!($fmt, ": {}"), $($arg),*, err);
            err
        }
    };
    ($err:expr, $fmt:literal, $($arg:expr),*) => {
        {
            warn!(concat!($fmt, ": {}"), $($arg),*, $err);
            $err
        }
    };
}

/// Logs an error at ERROR level for critical failures that may need attention.
#[macro_export]
macro_rules! log_error {
    ($fmt:literal, $($arg:expr),*) => {
        |err| {
            error!(concat!($fmt, ": {}"), $($arg),*, err);
            err
        }
    };
}

/// Tries an operation, logs any error at WARN level, and returns success indicator.
#[allow(dead_code)]
pub fn warn_on_error<E: std::fmt::Display>(result: Result<(), E>, context: &str) -> bool {
    match result {
        Ok(()) => true,
        Err(err) => {
            warn!("{}: {}", context, err);
            false
        }
    }
}

/// Tries an operation, logs any error at ERROR level, and returns success indicator.
#[allow(dead_code)]
pub fn error_on_failure<E: std::fmt::Display>(result: Result<(), E>, context: &str) -> bool {
    match result {
        Ok(()) => true,
        Err(err) => {
            error!("{}: {}", context, err);
            false
        }
    }
}

/// Logs a debug message for successful operations that may be useful for tracing.
#[macro_export]
macro_rules! log_success {
    ($fmt:literal, $($arg:expr),*) => {
        debug!($fmt, $($arg),*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warn_on_error_returns_true_on_ok() {
        let result: Result<(), &str> = Ok(());
        assert!(warn_on_error(result, "test"));
    }

    #[test]
    fn warn_on_error_returns_false_on_err() {
        let result: Result<(), &str> = Err("error message");
        assert!(!warn_on_error(result, "test"));
    }

    #[test]
    fn error_on_failure_returns_true_on_ok() {
        let result: Result<(), &str> = Ok(());
        assert!(error_on_failure(result, "test"));
    }

    #[test]
    fn error_on_failure_returns_false_on_err() {
        let result: Result<(), &str> = Err("error message");
        assert!(!error_on_failure(result, "test"));
    }
}
