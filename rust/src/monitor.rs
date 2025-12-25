use serde::Serialize;
use sysinfo::System;

#[derive(Serialize)]
pub struct StatusResponse {
    pub model_loaded: bool,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub state: String,
}

pub struct Monitor {
    system: System,
}

impl Monitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }

    pub fn get_status(&mut self, model_loaded: bool, state: &str) -> StatusResponse {
        self.system.refresh_all();
        let cpu_usage = self.system.global_cpu_info().cpu_usage();
        let memory_usage = self.system.used_memory();
        StatusResponse {
            model_loaded,
            cpu_usage,
            memory_usage,
            state: state.to_string(),
        }
    }
}
