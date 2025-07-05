use pacm_constants::SIMPLE_PACKAGES;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct SystemCapabilities {
    pub cpu_cores: usize,
    pub logical_cores: usize,
    pub available_memory_gb: f64,
    pub optimal_parallel_downloads: usize,
    pub optimal_parallel_resolutions: usize,
    pub optimal_cache_batch_size: usize,
    pub max_concurrent_network_requests: usize,
    pub optimal_dependency_batch_size: usize,
}

static SYSTEM_CAPS: OnceLock<SystemCapabilities> = OnceLock::new();

impl SystemCapabilities {
    pub fn get() -> &'static SystemCapabilities {
        SYSTEM_CAPS.get_or_init(|| {
            let cpu_cores = num_cpus::get_physical();
            let logical_cores = num_cpus::get();

            let available_memory_gb = Self::get_available_memory();

            let optimal_parallel_downloads = (logical_cores * 4).min(32).max(8);
            let optimal_parallel_resolutions = (logical_cores * 6).min(48).max(12);
            let optimal_cache_batch_size = (available_memory_gb * 200.0) as usize;
            let max_concurrent_network_requests = (logical_cores * 8).min(64).max(16);
            let optimal_dependency_batch_size = (logical_cores * 2).min(16).max(4);

            SystemCapabilities {
                cpu_cores,
                logical_cores,
                available_memory_gb,
                optimal_parallel_downloads,
                optimal_parallel_resolutions,
                optimal_cache_batch_size,
                max_concurrent_network_requests,
                optimal_dependency_batch_size,
            }
        })
    }

    fn get_available_memory() -> f64 {
        #[cfg(target_os = "windows")]
        {
            use std::mem;

            #[repr(C)]
            struct MemoryStatusEx {
                dw_length: u32,
                dw_memory_load: u32,
                ull_total_phys: u64,
                ull_avail_phys: u64,
                ull_total_page_file: u64,
                ull_avail_page_file: u64,
                ull_total_virtual: u64,
                ull_avail_virtual: u64,
                ull_avail_extended_virtual: u64,
            }

            unsafe extern "system" {
                fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
            }

            unsafe {
                let mut memory_status = MemoryStatusEx {
                    dw_length: mem::size_of::<MemoryStatusEx>() as u32,
                    dw_memory_load: 0,
                    ull_total_phys: 0,
                    ull_avail_phys: 0,
                    ull_total_page_file: 0,
                    ull_avail_page_file: 0,
                    ull_total_virtual: 0,
                    ull_avail_virtual: 0,
                    ull_avail_extended_virtual: 0,
                };

                if GlobalMemoryStatusEx(&mut memory_status) != 0 {
                    // Convert bytes to GB and use 50% of available memory
                    let available_gb =
                        (memory_status.ull_avail_phys as f64) / (1024.0 * 1024.0 * 1024.0);
                    (available_gb * 0.5).max(2.0).min(32.0)
                } else {
                    8.0 // Fallback for Windows
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemAvailable:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                let gb = (kb as f64) / (1024.0 * 1024.0);
                                return (gb * 0.5).max(2.0).min(32.0);
                            }
                        }
                    }
                }
            }
            4.0 // Fallback for Unix
        }
    }

    pub fn should_use_parallel_for_count(&self, count: usize) -> bool {
        count > 1 && count <= self.optimal_parallel_resolutions
    }

    pub fn get_optimal_batch_size(&self, total_items: usize) -> usize {
        if total_items <= 3 {
            return total_items;
        }

        let batch_size = (total_items / self.logical_cores).max(1).min(8);
        batch_size.min(total_items)
    }

    pub fn get_network_batch_size(&self, total_requests: usize) -> usize {
        if total_requests <= 4 {
            return total_requests;
        }

        let batch_size = self.max_concurrent_network_requests.min(total_requests);
        batch_size.max(4)
    }

    pub fn should_skip_transitive_analysis(&self, package_name: &str) -> bool {
        SIMPLE_PACKAGES.contains(&package_name)
            || package_name.starts_with("@types/")
            || package_name.contains("-utils")
            || package_name.contains("-helper")
            || package_name.contains("-tool")
            || package_name.contains("-cli")
            || package_name.len() < 6 // Very short names are usually simple
    }

    pub fn get_parallel_resolution_limit(&self) -> usize {
        if self.available_memory_gb > 16.0 {
            self.optimal_parallel_resolutions
        } else if self.available_memory_gb > 8.0 {
            self.optimal_parallel_resolutions / 2
        } else {
            self.optimal_parallel_resolutions / 4
        }
    }
}
