pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
pub const PROJECT_DIR: &str = env!("CARGO_MANIFEST_DIR");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub static ARGS: once_cell::sync::Lazy<crate::args::Args> = once_cell::sync::Lazy::new(clap::Parser::parse);

pub const DOMAINS_TO_INCLUDE: usize = 250;
pub const MAX_EPOLL_EVENTS_RETURNED: isize = 340;
pub const EPOLL_TIMEOUT_MILLIS: isize = 1000;
pub const MAX_CONCURRENCY: usize = 8;
pub const COLLECTION_TIMEOUT_MS: i64 = 2000;