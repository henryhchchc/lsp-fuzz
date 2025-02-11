//! Types representing sanitizer configuration options

/// Controls how colors are used in reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorOutput {
    Always,
    Never,
    Auto,
}

/// Controls how stack traces are formatted
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackTraceFormat {
    /// Default format
    Default,
    /// Custom format string
    Custom(String),
}

/// Controls how signal handlers are registered
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalHandling {
    /// Don't register handler
    None = 0,
    /// Register handler but allow user override
    RegisterAllowOverride = 1,
    /// Register handler and block changes
    RegisterBlockOverride = 2,
}

/// Controls how logs are written
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogPath {
    /// Write logs to stderr
    Stderr,
    /// Write logs to stdout
    Stdout,
    /// Write logs to a file, with `.{pid}` as a suffix
    File(String),
}

/// Common sanitizer options shared between different sanitizer types
#[derive(Debug, Clone)]
pub struct SanitizerOptions {
    /// If set, use the online symbolizer from common sanitizer runtime to turn virtual addresses to file/line locations
    pub symbolize: bool,

    /// Path to external symbolizer. If empty, the tool will search $PATH for the symbolizer
    pub external_symbolizer_path: Option<String>,

    /// If set, allows online symbolizer to run addr2line binary to symbolize stack traces
    pub allow_addr2line: bool,

    /// Strips this prefix from file paths in error reports
    pub strip_path_prefix: Option<String>,

    /// If available, use the fast frame-pointer-based unwinder on internal CHECK failures
    pub fast_unwind_on_check: bool,

    /// If available, use the fast frame-pointer-based unwinder on fatal errors
    pub fast_unwind_on_fatal: bool,

    /// If available, use the fast frame-pointer-based unwinder on malloc/free
    pub fast_unwind_on_malloc: bool,

    /// Intercept and handle ioctl requests
    pub handle_ioctl: bool,

    /// Max number of stack frames kept for each allocation/deallocation
    pub malloc_context_size: usize,

    /// Path to write log to
    pub log_path: LogPath,

    /// Mention name of executable when reporting error
    pub log_exe_name: bool,

    /// Write all sanitizer output to syslog in addition to other means of logging
    pub log_to_syslog: bool,

    /// Verbosity level (0 - silent, 1 - a bit of output, 2+ - more output)
    pub verbosity: usize,

    /// Enable memory leak detection
    pub detect_leaks: bool,

    /// Invoke leak checking in an atexit handler
    pub leak_check_at_exit: bool,

    /// If false, the allocator will crash instead of returning 0 on out-of-memory
    pub allocator_may_return_null: bool,

    /// If false, disable printing error summaries in addition to error reports
    pub print_summary: bool,

    /// Check printf arguments
    pub check_printf: bool,

    /// Controls custom tool's SEGV handler
    pub handle_segv: SignalHandling,

    /// Controls custom tool's SIGBUS handler
    pub handle_sigbus: SignalHandling,

    /// Controls custom tool's SIGABRT handler
    pub handle_abort: SignalHandling,

    /// Controls custom tool's SIGILL handler
    pub handle_sigill: SignalHandling,

    /// Controls custom tool's SIGFPE handler
    pub handle_sigfpe: SignalHandling,

    /// If set, uses alternate stack for signal handling
    pub use_sigaltstack: bool,

    /// If set, deadlock detection is enabled
    pub detect_deadlocks: bool,

    /// Large shadow regions zero-fill threshold in bytes
    pub clear_shadow_mmap_threshold: usize,

    /// Colorize reports settings
    pub color: ColorOutput,

    /// Enables support for dynamic libraries linked with libpthread 2.2.5
    pub legacy_pthread_cond: bool,

    /// Intercept `__tls_get_addr`
    pub intercept_tls_get_addr: bool,

    /// Limit the amount of mmap-ed memory (excluding shadow) in Mb
    pub mmap_limit_mb: usize,

    /// Hard RSS limit in Mb
    pub hard_rss_limit_mb: usize,

    /// Soft RSS limit in Mb
    pub soft_rss_limit_mb: usize,

    /// If false, do not attempt to read /proc/maps/statm
    pub can_use_proc_maps_statm: bool,

    /// If set, coverage information will be dumped at program shutdown
    pub coverage: bool,

    /// If set, the coverage information will be dumped as PC offsets
    pub coverage_pcs: bool,

    /// If true, the PCs will be dumped in execution order
    pub coverage_order_pcs: bool,

    /// If set, coverage information will also be dumped as a bitset
    pub coverage_bitset: bool,

    /// If set, the bitmap for coverage counters will be dumped
    pub coverage_counters: bool,

    /// If set, coverage information will be dumped directly to a memory mapped file
    pub coverage_direct: bool,

    /// Target directory for coverage dumps
    pub coverage_dir: String,

    /// Sanitize complete address space
    pub full_address_space: bool,

    /// Print matched suppressions at exit
    pub print_suppressions: bool,

    /// Disable core dumping
    pub disable_coredump: bool,

    /// If set, instructs kernel to not store the shadow in core file
    pub use_madv_dontdump: bool,

    /// Print inlined frames in stacktraces
    pub symbolize_inline_frames: bool,

    /// Print file locations in Visual Studio style
    pub symbolize_vs_style: bool,

    /// Format string used to render stack frames
    pub stack_trace_format: StackTraceFormat,

    /// If true, the shadow is not allowed to use huge pages
    pub no_huge_pages_for_shadow: bool,

    /// If set check that string arguments are properly null-terminated
    pub strict_string_checks: bool,

    /// Use custom wrappers for strstr and strcasestr
    pub intercept_strstr: bool,

    /// Use custom wrappers for strspn and strcspn
    pub intercept_strspn: bool,

    /// Use custom wrappers for strpbrk
    pub intercept_strpbrk: bool,

    /// Use custom wrappers for memcmp
    pub intercept_memcmp: bool,

    /// Assume memcmp always reads n bytes before comparing
    pub strict_memcmp: bool,

    /// Decorate sanitizer mappings in /proc/self/maps
    pub decorate_proc_maps: bool,

    /// Program exit status if tool found an error
    pub exitcode: i32,

    /// Call abort() instead of _exit() after error report
    pub abort_on_error: bool,

    /// Additional options file path
    pub include: Option<String>,

    /// Additional options file path (if exists)
    pub include_if_exists: Option<String>,

    /// Deduplicate reports for single source location
    pub suppress_equal_pcs: bool,

    /// Print command line on crash
    pub print_cmdline: bool,

    /// Enable experimental heap profiler
    pub heap_profile: bool,

    /// Interval for releasing memory to OS (ms)
    pub allocator_release_to_os_interval_ms: i64,

    /// Generate html coverage report
    pub html_cov_report: bool,

    /// Sancov tool location
    pub sancov_path: String,
}

impl Default for SanitizerOptions {
    fn default() -> Self {
        Self {
            symbolize: true,
            external_symbolizer_path: None,
            allow_addr2line: false,
            strip_path_prefix: None,
            fast_unwind_on_check: false,
            fast_unwind_on_fatal: false,
            fast_unwind_on_malloc: true,
            handle_ioctl: false,
            malloc_context_size: 30,
            log_path: LogPath::Stderr,
            log_exe_name: false,
            log_to_syslog: false,
            verbosity: 0,
            detect_leaks: true,
            leak_check_at_exit: true,
            allocator_may_return_null: false,
            print_summary: true,
            check_printf: true,
            handle_segv: SignalHandling::RegisterAllowOverride,
            handle_sigbus: SignalHandling::RegisterAllowOverride,
            handle_abort: SignalHandling::None,
            handle_sigill: SignalHandling::RegisterAllowOverride,
            handle_sigfpe: SignalHandling::RegisterAllowOverride,
            use_sigaltstack: true,
            detect_deadlocks: true,
            clear_shadow_mmap_threshold: 64 * 1024,
            color: ColorOutput::Auto,
            legacy_pthread_cond: false,
            intercept_tls_get_addr: false,
            mmap_limit_mb: 0,
            hard_rss_limit_mb: 0,
            soft_rss_limit_mb: 0,
            can_use_proc_maps_statm: true,
            coverage: false,
            coverage_pcs: true,
            coverage_order_pcs: false,
            coverage_bitset: false,
            coverage_counters: false,
            coverage_direct: false,
            coverage_dir: ".".to_string(),
            full_address_space: false,
            print_suppressions: true,
            disable_coredump: true,
            use_madv_dontdump: true,
            symbolize_inline_frames: true,
            symbolize_vs_style: false,
            stack_trace_format: StackTraceFormat::Default,
            no_huge_pages_for_shadow: true,
            strict_string_checks: false,
            intercept_strstr: true,
            intercept_strspn: true,
            intercept_strpbrk: true,
            intercept_memcmp: true,
            strict_memcmp: true,
            decorate_proc_maps: false,
            exitcode: 1,
            abort_on_error: false,
            include: None,
            include_if_exists: None,
            suppress_equal_pcs: true,
            print_cmdline: false,
            heap_profile: false,
            allocator_release_to_os_interval_ms: -1,
            html_cov_report: false,
            sancov_path: "sancov".to_string(),
        }
    }
}
