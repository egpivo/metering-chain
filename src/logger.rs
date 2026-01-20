/// Simple logger for metering-chain
pub struct Logger;

impl Logger {
    pub fn info(msg: &str) {
        println!("[INFO] {}", msg);
    }

    pub fn debug(msg: &str) {
        println!("[DEBUG] {}", msg);
    }

    pub fn warn(msg: &str) {
        eprintln!("[WARN] {}", msg);
    }

    pub fn error(msg: &str) {
        eprintln!("[ERROR] {}", msg);
    }
}
