//! One stats sample for the selected container.

use bollard::models::ContainerStatsResponse;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContainerStats {
    pub id: String,
    /// CPU use as a share of one core times 100, as `docker stats` shows it.
    pub cpu_percent: Option<f64>,
    /// Memory in use, without the reclaimable page cache.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub memory_bytes: Option<u64>,
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub memory_limit: Option<u64>,
}

/// Computes CPU and memory the way `docker stats` does.
pub fn sample(id: &str, s: &ContainerStatsResponse) -> ContainerStats {
    let cpu = s.cpu_stats.as_ref();
    let pre = s.precpu_stats.as_ref();
    let total = |c: Option<&bollard::models::ContainerCpuStats>| {
        c.and_then(|c| c.cpu_usage.as_ref())
            .and_then(|u| u.total_usage)
    };
    let cpu_percent = match (
        total(cpu),
        total(pre),
        cpu.and_then(|c| c.system_cpu_usage),
        pre.and_then(|c| c.system_cpu_usage),
    ) {
        (Some(t), Some(pt), Some(sys), Some(psys)) if sys > psys && t >= pt => {
            let cpus = cpu
                .and_then(|c| c.online_cpus)
                .filter(|n| *n > 0)
                .map(u64::from)
                .or_else(|| {
                    cpu.and_then(|c| c.cpu_usage.as_ref())
                        .and_then(|u| u.percpu_usage.as_ref())
                        .map(|v| v.len() as u64)
                })
                .unwrap_or(1);
            #[allow(clippy::cast_precision_loss)]
            let pct = (t - pt) as f64 / (sys - psys) as f64 * cpus as f64 * 100.0;
            Some((pct * 10.0).round() / 10.0)
        }
        _ => None,
    };
    let mem = s.memory_stats.as_ref();
    let memory_bytes = mem.and_then(|m| m.usage).map(|usage| {
        let cache = mem
            .and_then(|m| m.stats.as_ref())
            .and_then(|st| {
                st.get("inactive_file")
                    .or_else(|| st.get("total_inactive_file"))
                    .or_else(|| st.get("cache"))
                    .copied()
            })
            .unwrap_or(0);
        usage.saturating_sub(cache)
    });
    ContainerStats {
        id: id.to_string(),
        cpu_percent,
        memory_bytes,
        memory_limit: mem.and_then(|m| m.limit).filter(|l| *l > 0),
    }
}
