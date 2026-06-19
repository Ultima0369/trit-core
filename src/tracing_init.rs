//! Tracing initialization for Trit-Core binaries.
//!
//! Provides structured logging via `tracing-subscriber`.
//! Controlled by the `TRIT_LOG` environment variable (default: `info`).
//!
//! # Environment variables
//! - `TRIT_LOG`: log filter (e.g. `debug`, `info`, `warn`, `trit_core=debug`)
//!   Falls back to `RUST_LOG` if `TRIT_LOG` is not set.
//! - `TRIT_LOG_JSON`: set to `0` or `false` for human-readable output instead of JSON.
//! - `TRIT_LOG_FILE`: path to write logs to a file instead of stderr.
//! - `TRIT_LOG_FORMAT`: one of `json`, `pretty`, `compact`, `full`.
//!
//! # Programmatic control
//! Use [`init_with_opts`] to configure logging without environment variables.

use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Pretty human-readable multi-line format.
    Pretty,
    /// Compact single-line format.
    Compact,
    /// Full human-readable format.
    Full,
    /// JSON structured format.
    #[default]
    Json,
}

impl std::str::FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Ok(LogFormat::Json),
            "pretty" => Ok(LogFormat::Pretty),
            "compact" => Ok(LogFormat::Compact),
            "full" => Ok(LogFormat::Full),
            other => Err(format!(
                "unknown log format '{}': expected json|pretty|compact|full",
                other
            )),
        }
    }
}

/// Options for initializing the tracing subscriber.
#[derive(Debug, Clone)]
pub struct LogOptions {
    /// Log filter directive (e.g. "info", "debug").
    pub filter: String,
    /// Output format.
    pub format: LogFormat,
    /// Optional path to write logs to a file.
    pub file: Option<std::path::PathBuf>,
    /// Whether to include span close events.
    pub span_events: bool,
}

impl Default for LogOptions {
    fn default() -> Self {
        Self {
            filter: "info".to_string(),
            format: LogFormat::Json,
            file: None,
            span_events: true,
        }
    }
}

impl LogOptions {
    /// Read options from environment variables.
    pub fn from_env() -> Self {
        let filter = std::env::var("TRIT_LOG")
            .or_else(|_| std::env::var("RUST_LOG"))
            .unwrap_or_else(|_| "info".to_string());

        let use_json = std::env::var("TRIT_LOG_JSON")
            .map(|v| v != "0" && v != "false" && v != "off")
            .unwrap_or(true);

        let format = std::env::var("TRIT_LOG_FORMAT")
            .ok()
            .and_then(|v| v.parse::<LogFormat>().ok())
            .unwrap_or(if use_json {
                LogFormat::Json
            } else {
                LogFormat::Full
            });

        let file = std::env::var("TRIT_LOG_FILE")
            .ok()
            .map(std::path::PathBuf::from);

        Self {
            filter,
            format,
            file,
            span_events: true,
        }
    }

    /// Set the log level filter.
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = filter.into();
        self
    }

    /// Set the output format.
    pub fn with_format(mut self, format: LogFormat) -> Self {
        self.format = format;
        self
    }

    /// Set an optional log file path.
    pub fn with_file(mut self, path: impl AsRef<Path>) -> Self {
        self.file = Some(path.as_ref().to_path_buf());
        self
    }
}

/// Thread-safe file writer wrapper.
#[derive(Clone)]
struct ArcWriter {
    inner: Arc<Mutex<std::fs::File>>,
}

impl ArcWriter {
    fn new(file: std::fs::File) -> Self {
        Self {
            inner: Arc::new(Mutex::new(file)),
        }
    }
}

impl Write for ArcWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| io::Error::other(format!("log file mutex poisoned: {}", e)))?;
        guard.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| io::Error::other(format!("log file mutex poisoned: {}", e)))?;
        guard.flush()
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for ArcWriter {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn build_env_filter(opts: &LogOptions) -> Result<EnvFilter, String> {
    EnvFilter::try_new(&opts.filter)
        .map_err(|e| format!("invalid log filter '{}': {}", opts.filter, e))
}

/// Initialize the tracing subscriber with options.
///
/// Returns an error if the filter directive is invalid or the log file cannot be opened.
pub fn init_with_opts(opts: LogOptions) -> Result<(), String> {
    let env_filter = build_env_filter(&opts)?;

    if let Some(path) = &opts.file {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("failed to open log file '{}': {}", path.display(), e))?;
        let writer = ArcWriter::new(file);
        init_file_subscriber(env_filter, writer, opts.format, opts.span_events)?;
    } else {
        init_stderr_subscriber(env_filter, opts.format, opts.span_events)?;
    }

    Ok(())
}

fn init_file_subscriber<W>(
    env_filter: EnvFilter,
    writer: W,
    format: LogFormat,
    span_events: bool,
) -> Result<(), String>
where
    W: for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync + 'static,
{
    let span_events_opt = if span_events {
        FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    match format {
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .json()
                .with_span_events(span_events_opt)
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_writer(writer)
                .try_init()
                .map_err(|e| format!("tracing subscriber already initialized: {}", e))?;
        }
        LogFormat::Pretty => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .pretty()
                .with_span_events(span_events_opt)
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_writer(writer)
                .try_init()
                .map_err(|e| format!("tracing subscriber already initialized: {}", e))?;
        }
        LogFormat::Compact => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .compact()
                .with_span_events(span_events_opt)
                .with_target(true)
                .with_writer(writer)
                .try_init()
                .map_err(|e| format!("tracing subscriber already initialized: {}", e))?;
        }
        LogFormat::Full => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_span_events(span_events_opt)
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_writer(writer)
                .try_init()
                .map_err(|e| format!("tracing subscriber already initialized: {}", e))?;
        }
    }

    Ok(())
}

fn init_stderr_subscriber(
    env_filter: EnvFilter,
    format: LogFormat,
    span_events: bool,
) -> Result<(), String> {
    init_file_subscriber(env_filter, io::stderr, format, span_events)
}

/// Initialize the tracing subscriber using environment variables.
///
/// Prints a warning to stderr if initialization fails.
pub fn init() {
    if let Err(e) = init_with_opts(LogOptions::from_env()) {
        eprintln!("[trit-core] warning: failed to initialize tracing: {}", e);
    }
}
