#[cfg(windows)]
mod service;

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    ping_service::run()
}

#[cfg(not(windows))]
fn main() {
    panic!("This program is only intended to run on Windows.");
}
