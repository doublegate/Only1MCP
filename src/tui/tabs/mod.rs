//! Tab implementations

pub mod cache;
pub mod logs;
pub mod overview;
pub mod requests;
pub mod servers;

#[derive(Clone, Copy, PartialEq)]
pub enum TabId {
    Overview,
    Servers,
    Requests,
    Cache,
    Logs,
}
