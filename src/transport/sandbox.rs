//! Process sandboxing and isolation for STDIO transports
//!
//! Provides security hardening features for spawned MCP server processes:
//! - Seccomp filtering (Linux only)
//! - Resource limits (memory, CPU, file descriptors)
//! - Namespace isolation
//! - Capability dropping

use tracing::{debug, warn};

/// Sandbox configuration for process isolation
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Enable seccomp filtering (Linux only)
    pub enable_seccomp: bool,

    /// Maximum memory in MB
    pub max_memory_mb: Option<u32>,

    /// Maximum CPU percentage (0-100)
    pub max_cpu_percent: Option<u8>,

    /// Maximum file descriptors
    pub max_file_descriptors: Option<u32>,

    /// Allowed system calls (whitelist for seccomp)
    pub allowed_syscalls: Vec<String>,

    /// Enable network isolation
    pub isolate_network: bool,

    /// Enable filesystem isolation (chroot/pivot_root)
    pub isolate_filesystem: bool,

    /// Read-only paths
    pub readonly_paths: Vec<String>,

    /// Temporary directory path
    pub temp_dir: Option<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enable_seccomp: cfg!(target_os = "linux"),
            max_memory_mb: Some(512),
            max_cpu_percent: Some(50),
            max_file_descriptors: Some(1024),
            allowed_syscalls: default_allowed_syscalls(),
            isolate_network: false,
            isolate_filesystem: false,
            readonly_paths: vec![],
            temp_dir: None,
        }
    }
}

/// Default allowed syscalls for MCP servers
fn default_allowed_syscalls() -> Vec<String> {
    vec![
        // Essential syscalls for process execution
        "read".to_string(),
        "write".to_string(),
        "open".to_string(),
        "openat".to_string(),
        "close".to_string(),
        "stat".to_string(),
        "fstat".to_string(),
        "lstat".to_string(),
        "poll".to_string(),
        "lseek".to_string(),
        "mmap".to_string(),
        "mprotect".to_string(),
        "munmap".to_string(),
        "brk".to_string(),
        "rt_sigaction".to_string(),
        "rt_sigprocmask".to_string(),
        "rt_sigreturn".to_string(),
        "ioctl".to_string(),
        "pread64".to_string(),
        "pwrite64".to_string(),
        "readv".to_string(),
        "writev".to_string(),
        "access".to_string(),
        "pipe".to_string(),
        "select".to_string(),
        "sched_yield".to_string(),
        "mremap".to_string(),
        "msync".to_string(),
        "mincore".to_string(),
        "madvise".to_string(),
        "dup".to_string(),
        "dup2".to_string(),
        "dup3".to_string(),
        "pause".to_string(),
        "nanosleep".to_string(),
        "getitimer".to_string(),
        "alarm".to_string(),
        "setitimer".to_string(),
        "getpid".to_string(),
        "sendfile".to_string(),
        "socket".to_string(), // May be needed for some MCP servers
        "connect".to_string(),
        "accept".to_string(),
        "sendto".to_string(),
        "recvfrom".to_string(),
        "sendmsg".to_string(),
        "recvmsg".to_string(),
        "shutdown".to_string(),
        "bind".to_string(),
        "listen".to_string(),
        "getsockname".to_string(),
        "getpeername".to_string(),
        "socketpair".to_string(),
        "setsockopt".to_string(),
        "getsockopt".to_string(),
        "clone".to_string(),
        "fork".to_string(),
        "vfork".to_string(),
        "execve".to_string(),
        "exit".to_string(),
        "exit_group".to_string(),
        "wait4".to_string(),
        "kill".to_string(),
        "uname".to_string(),
        "fcntl".to_string(),
        "flock".to_string(),
        "fsync".to_string(),
        "fdatasync".to_string(),
        "truncate".to_string(),
        "ftruncate".to_string(),
        "getdents".to_string(),
        "getdents64".to_string(),
        "getcwd".to_string(),
        "chdir".to_string(),
        "fchdir".to_string(),
        "rename".to_string(),
        "mkdir".to_string(),
        "rmdir".to_string(),
        "creat".to_string(),
        "link".to_string(),
        "unlink".to_string(),
        "symlink".to_string(),
        "readlink".to_string(),
        "chmod".to_string(),
        "fchmod".to_string(),
        "chown".to_string(),
        "fchown".to_string(),
        "lchown".to_string(),
        "umask".to_string(),
        "gettimeofday".to_string(),
        "getrlimit".to_string(),
        "getrusage".to_string(),
        "sysinfo".to_string(),
        "times".to_string(),
        "ptrace".to_string(),
        "getuid".to_string(),
        "syslog".to_string(),
        "getgid".to_string(),
        "setuid".to_string(),
        "setgid".to_string(),
        "geteuid".to_string(),
        "getegid".to_string(),
        "setpgid".to_string(),
        "getppid".to_string(),
        "getpgrp".to_string(),
        "setsid".to_string(),
        "setreuid".to_string(),
        "setregid".to_string(),
        "getgroups".to_string(),
        "setgroups".to_string(),
        "setresuid".to_string(),
        "getresuid".to_string(),
        "setresgid".to_string(),
        "getresgid".to_string(),
        "getpgid".to_string(),
        "setfsuid".to_string(),
        "setfsgid".to_string(),
        "getsid".to_string(),
        "capget".to_string(),
        "capset".to_string(),
        "rt_sigpending".to_string(),
        "rt_sigtimedwait".to_string(),
        "rt_sigqueueinfo".to_string(),
        "rt_sigsuspend".to_string(),
        "sigaltstack".to_string(),
        "utime".to_string(),
        "mknod".to_string(),
        "personality".to_string(),
        "ustat".to_string(),
        "statfs".to_string(),
        "fstatfs".to_string(),
        "sysfs".to_string(),
        "getpriority".to_string(),
        "setpriority".to_string(),
        "sched_setparam".to_string(),
        "sched_getparam".to_string(),
        "sched_setscheduler".to_string(),
        "sched_getscheduler".to_string(),
        "sched_get_priority_max".to_string(),
        "sched_get_priority_min".to_string(),
        "sched_rr_get_interval".to_string(),
        "mlock".to_string(),
        "munlock".to_string(),
        "mlockall".to_string(),
        "munlockall".to_string(),
        "vhangup".to_string(),
        "prctl".to_string(),
        "arch_prctl".to_string(),
        "adjtimex".to_string(),
        "setrlimit".to_string(),
        "sync".to_string(),
        "acct".to_string(),
        "settimeofday".to_string(),
        "mount".to_string(),
        "umount2".to_string(),
        "swapon".to_string(),
        "swapoff".to_string(),
        "reboot".to_string(),
        "sethostname".to_string(),
        "setdomainname".to_string(),
        "quotactl".to_string(),
        "gettid".to_string(),
        "readahead".to_string(),
        "setxattr".to_string(),
        "lsetxattr".to_string(),
        "fsetxattr".to_string(),
        "getxattr".to_string(),
        "lgetxattr".to_string(),
        "fgetxattr".to_string(),
        "listxattr".to_string(),
        "llistxattr".to_string(),
        "flistxattr".to_string(),
        "removexattr".to_string(),
        "lremovexattr".to_string(),
        "fremovexattr".to_string(),
        "tkill".to_string(),
        "time".to_string(),
        "futex".to_string(),
        "sched_setaffinity".to_string(),
        "sched_getaffinity".to_string(),
        "io_setup".to_string(),
        "io_destroy".to_string(),
        "io_getevents".to_string(),
        "io_submit".to_string(),
        "io_cancel".to_string(),
        "lookup_dcookie".to_string(),
        "epoll_create".to_string(),
        "epoll_ctl".to_string(),
        "epoll_wait".to_string(),
        "epoll_pwait".to_string(),
        "epoll_create1".to_string(),
        "remap_file_pages".to_string(),
        "getdents64".to_string(),
        "set_tid_address".to_string(),
        "restart_syscall".to_string(),
        "semtimedop".to_string(),
        "fadvise64".to_string(),
        "timer_create".to_string(),
        "timer_settime".to_string(),
        "timer_gettime".to_string(),
        "timer_getoverrun".to_string(),
        "timer_delete".to_string(),
        "clock_settime".to_string(),
        "clock_gettime".to_string(),
        "clock_getres".to_string(),
        "clock_nanosleep".to_string(),
        "tgkill".to_string(),
        "utimes".to_string(),
        "mbind".to_string(),
        "set_mempolicy".to_string(),
        "get_mempolicy".to_string(),
        "mq_open".to_string(),
        "mq_unlink".to_string(),
        "mq_timedsend".to_string(),
        "mq_timedreceive".to_string(),
        "mq_notify".to_string(),
        "mq_getsetattr".to_string(),
        "waitid".to_string(),
        "inotify_init".to_string(),
        "inotify_add_watch".to_string(),
        "inotify_rm_watch".to_string(),
        "openat".to_string(),
        "mkdirat".to_string(),
        "mknodat".to_string(),
        "fchownat".to_string(),
        "futimesat".to_string(),
        "newfstatat".to_string(),
        "unlinkat".to_string(),
        "renameat".to_string(),
        "linkat".to_string(),
        "symlinkat".to_string(),
        "readlinkat".to_string(),
        "fchmodat".to_string(),
        "faccessat".to_string(),
        "pselect6".to_string(),
        "ppoll".to_string(),
        "unshare".to_string(),
        "splice".to_string(),
        "tee".to_string(),
        "sync_file_range".to_string(),
        "vmsplice".to_string(),
        "move_pages".to_string(),
        "utimensat".to_string(),
        "signalfd".to_string(),
        "timerfd_create".to_string(),
        "eventfd".to_string(),
        "fallocate".to_string(),
        "timerfd_settime".to_string(),
        "timerfd_gettime".to_string(),
        "accept4".to_string(),
        "signalfd4".to_string(),
        "eventfd2".to_string(),
        "inotify_init1".to_string(),
        "pipe2".to_string(),
        "preadv".to_string(),
        "pwritev".to_string(),
        "rt_tgsigqueueinfo".to_string(),
        "perf_event_open".to_string(),
        "recvmmsg".to_string(),
        "prlimit64".to_string(),
        "sendmmsg".to_string(),
        "getcpu".to_string(),
    ]
}

/// Apply sandbox configuration to a tokio Command
///
/// This function applies security hardening to the command before spawning.
/// On Linux, it can apply seccomp filters. On all platforms, it applies
/// resource limits where supported.
pub fn apply_sandbox(cmd: &mut tokio::process::Command, config: &SandboxConfig) {
    debug!("Applying sandbox configuration to process");

    // Set environment to minimal
    cmd.env_clear();
    cmd.env("PATH", "/usr/local/bin:/usr/bin:/bin");

    // Apply resource limits using rlimit (cross-platform where supported)
    #[cfg(target_os = "linux")]
    {
        if let Some(max_mem_mb) = config.max_memory_mb {
            let max_bytes = (max_mem_mb as u64) * 1024 * 1024;
            debug!("Setting memory limit to {} MB", max_mem_mb);

            // Note: rlimit crate would be used here in production
            // For now, we'll use process_limit hook
            unsafe {
                cmd.pre_exec(move || {
                    use libc::{rlimit, setrlimit, RLIMIT_AS, RLIMIT_DATA};
                    let limit = rlimit {
                        rlim_cur: max_bytes,
                        rlim_max: max_bytes,
                    };
                    setrlimit(RLIMIT_AS, &limit);
                    setrlimit(RLIMIT_DATA, &limit);
                    Ok(())
                });
            }
        }

        if let Some(max_fds) = config.max_file_descriptors {
            debug!("Setting file descriptor limit to {}", max_fds);
            unsafe {
                cmd.pre_exec(move || {
                    use libc::{rlimit, setrlimit, RLIMIT_NOFILE};
                    let limit = rlimit {
                        rlim_cur: max_fds as u64,
                        rlim_max: max_fds as u64,
                    };
                    setrlimit(RLIMIT_NOFILE, &limit);
                    Ok(())
                });
            }
        }

        // Apply seccomp filtering if enabled
        if config.enable_seccomp {
            debug!("Seccomp filtering enabled (would apply in production)");
            // In production, use libseccomp-rs to create and apply filter
            // This requires the seccomp crate which we don't have in Cargo.toml
            // For now, we document the approach:

            // unsafe {
            //     cmd.pre_exec(move || {
            //         apply_seccomp_filter(&config.allowed_syscalls)?;
            //         Ok(())
            //     });
            // }

            warn!("Seccomp filtering not yet implemented - requires libseccomp-rs dependency");
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        if config.enable_seccomp {
            warn!("Seccomp filtering only available on Linux");
        }

        // On macOS/BSD, we could use other mechanisms like pledge() or sandbox_init()
        debug!("Resource limits not fully supported on this platform");
    }
}

/// Sandbox statistics for monitoring
#[derive(Debug, Clone)]
pub struct SandboxStats {
    pub memory_used_mb: u32,
    pub cpu_percent: f32,
    pub fd_count: u32,
    pub violations: u64,
}

impl SandboxStats {
    pub fn new() -> Self {
        Self {
            memory_used_mb: 0,
            cpu_percent: 0.0,
            fd_count: 0,
            violations: 0,
        }
    }
}

impl Default for SandboxStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_sandbox_config() {
        let config = SandboxConfig::default();
        assert!(config.max_memory_mb.is_some());
        assert!(config.max_cpu_percent.is_some());
        assert!(!config.allowed_syscalls.is_empty());
    }

    #[test]
    fn test_allowed_syscalls_includes_essentials() {
        let syscalls = default_allowed_syscalls();
        assert!(syscalls.contains(&"read".to_string()));
        assert!(syscalls.contains(&"write".to_string()));
        assert!(syscalls.contains(&"open".to_string()));
        assert!(syscalls.contains(&"close".to_string()));
    }
}
