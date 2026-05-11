mod bus;

pub use bus::EventBus;

use tspm_core::{TspmEvent, StopReason, RestartReason, SystemStopReason, LogType, ProcessState};

/// Create a ProcessStart event
pub fn event_process_start(name: &str, instance_id: u32, pid: Option<u32>) -> TspmEvent {
    TspmEvent::ProcessStart {
        name: name.to_string(),
        instance_id,
        pid,
    }
}

/// Create a ProcessStop event
pub fn event_process_stop(name: &str, instance_id: u32, pid: Option<u32>, reason: StopReason) -> TspmEvent {
    TspmEvent::ProcessStop {
        name: name.to_string(),
        instance_id,
        pid,
        reason,
    }
}

/// Create a ProcessRestart event
pub fn event_process_restart(
    name: &str,
    instance_id: u32,
    restart_count: u32,
    delay_ms: Option<u64>,
    reason: Option<RestartReason>,
) -> TspmEvent {
    TspmEvent::ProcessRestart {
        name: name.to_string(),
        instance_id,
        restart_count,
        delay_ms,
        reason,
    }
}

/// Create a ProcessExit event
pub fn event_process_exit(name: &str, instance_id: u32, exit_code: Option<i32>, signal: Option<i32>) -> TspmEvent {
    TspmEvent::ProcessExit {
        name: name.to_string(),
        instance_id,
        exit_code,
        signal,
    }
}

/// Create a ProcessError event
pub fn event_process_error(name: &str, instance_id: u32, error: &str) -> TspmEvent {
    TspmEvent::ProcessError {
        name: name.to_string(),
        instance_id,
        error: error.to_string(),
    }
}

/// Create a ProcessStateChange event
pub fn event_process_state_change(
    name: &str,
    instance_id: u32,
    previous: ProcessState,
    current: ProcessState,
) -> TspmEvent {
    TspmEvent::ProcessStateChange {
        name: name.to_string(),
        instance_id,
        previous,
        current,
    }
}

/// Create a ProcessLog event
pub fn event_process_log(name: &str, instance_id: u32, message: &str, log_type: LogType) -> TspmEvent {
    TspmEvent::ProcessLog {
        name: name.to_string(),
        instance_id,
        message: message.to_string(),
        log_type,
    }
}

/// Create a ProcessOom event
pub fn event_process_oom(name: &str, instance_id: u32, memory_bytes: u64, limit_bytes: u64) -> TspmEvent {
    TspmEvent::ProcessOom {
        name: name.to_string(),
        instance_id,
        memory_bytes,
        limit_bytes,
    }
}

/// Create a ProcessReady event
pub fn event_process_ready(name: &str, instance_id: u32, pid: Option<u32>) -> TspmEvent {
    TspmEvent::ProcessReady {
        name: name.to_string(),
        instance_id,
        pid,
    }
}

/// Create a SystemStart event
pub fn event_system_start(config_file: &str, process_count: usize) -> TspmEvent {
    TspmEvent::SystemStart {
        config_file: config_file.to_string(),
        process_count,
    }
}

/// Create a SystemStop event
pub fn event_system_stop(reason: SystemStopReason, graceful: bool) -> TspmEvent {
    TspmEvent::SystemStop { reason, graceful }
}

/// Create a SystemError event
pub fn event_system_error(error: &str) -> TspmEvent {
    TspmEvent::SystemError {
        error: error.to_string(),
    }
}
