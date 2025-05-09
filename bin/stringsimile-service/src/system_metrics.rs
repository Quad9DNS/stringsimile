use std::collections::HashMap;

use metrics::{Gauge, gauge};
use sysinfo::{System, get_current_pid};

pub struct SystemMetrics {
    system: System,

    host_total_memory: Gauge,
    host_used_memory: Gauge,
    host_cpu_count: Gauge,
    host_cpu_usages: HashMap<String, Gauge>,

    service_used_memory: Gauge,
    service_cpu_usage: Gauge,
}

impl SystemMetrics {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let host_cpu_usages = system
            .cpus()
            .iter()
            .map(|cpu| {
                (
                    cpu.name().to_string(),
                    gauge!("host_cpu_usage", "name" => cpu.name().to_string()),
                )
            })
            .collect();
        Self {
            system,
            host_total_memory: gauge!("host_total_memory"),
            host_used_memory: gauge!("host_used_memory"),
            host_cpu_count: gauge!("host_cpu_count"),
            host_cpu_usages,
            service_used_memory: gauge!("service_used_memory"),
            service_cpu_usage: gauge!("service_cpu_usage"),
        }
    }

    pub fn emit_system_metrics(&mut self) {
        self.system.refresh_all();

        self.host_total_memory
            .set(self.system.total_memory() as f64);
        self.host_used_memory.set(self.system.used_memory() as f64);
        self.host_cpu_count.set(self.system.cpus().len() as f64);

        for cpu in self.system.cpus() {
            if let Some(usage_gauge) = self.host_cpu_usages.get(cpu.name()) {
                usage_gauge.set(cpu.cpu_usage());
            }
        }

        self.emit_service_process_metrics();
    }

    fn emit_service_process_metrics(&mut self) {
        let Ok(pid) = get_current_pid() else {
            // Can't get process PID, can't do much more right now
            return;
        };
        let Some(process) = self.system.process(pid) else {
            // Can't find the process, can't do much more right now
            return;
        };

        self.service_cpu_usage.set(process.cpu_usage());
        self.service_used_memory.set(process.memory() as f64);
    }
}
