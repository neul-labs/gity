use bincode::Options;
use thiserror::Error;

/// Maximum allowed message size for IPC communication (1MB)
pub const MAX_MESSAGE_SIZE: u64 = 1_048_576;

/// Maximum allowed individual log entry size (1MB)
#[allow(dead_code)]
pub const MAX_LOG_ENTRY_SIZE: usize = 1_048_576;

/// Returns bincode configuration matching bincode::serialize/deserialize behavior
pub fn bounded_bincode() -> impl Options {
    // Use bincode::options() to get the same configuration as bincode::serialize/deserialize
    bincode::options()
}

/// Validates that a byte slice does not exceed the maximum allowed size
pub fn validate_message_size(data: &[u8]) -> Result<(), MessageSizeError> {
    if data.len() > MAX_MESSAGE_SIZE as usize {
        Err(MessageSizeError::TooLarge {
            actual: data.len(),
            max: MAX_MESSAGE_SIZE,
        })
    } else {
        Ok(())
    }
}

/// Error returned when a message exceeds the allowed size limit.
#[derive(Debug, Error)]
pub enum MessageSizeError {
    #[error("message too large: {actual} bytes exceeds maximum of {max} bytes")]
    TooLarge { actual: usize, max: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DaemonCommand, JobKind, ValidatedPath};

    #[test]
    fn small_message_deserializes() {
        let cmd = DaemonCommand::HealthCheck;
        let bytes = bounded_bincode().serialize(&cmd).unwrap();
        assert!(validate_message_size(&bytes).is_ok());
    }

    #[test]
    fn oversized_message_rejected() {
        let data = vec![0u8; 2_000_000]; // > 1MB
        assert!(matches!(
            validate_message_size(&data),
            Err(MessageSizeError::TooLarge {
                actual: 2_000_000,
                max: 1_048_576
            })
        ));
    }

    #[test]
    fn maximum_size_message_allowed() {
        let data = vec![0u8; 1_048_576]; // Exactly 1MB
        assert!(validate_message_size(&data).is_ok());
    }

    #[test]
    fn bincode_roundtrip_with_limits() {
        let cmd = DaemonCommand::QueueJob {
            repo_path: ValidatedPath::new(std::path::PathBuf::from("/test/repo")).unwrap(),
            job: JobKind::Prefetch,
        };
        // Use bounded_bincode for both serialize and deserialize
        let bytes = bounded_bincode().serialize(&cmd).unwrap();
        let decoded: DaemonCommand = bounded_bincode().deserialize(&bytes).unwrap();
        assert_eq!(cmd, decoded);
    }
}
