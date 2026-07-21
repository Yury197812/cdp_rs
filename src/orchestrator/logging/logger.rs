// orchestrator/logging/logger.rs - Structured logging
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Utc;

pub struct Logger {
    level: LogLevel,
    file: Option<std::fs::File>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Logger { level, file: None }
    }
    
    pub fn with_file(mut self, path: &str) -> Result<Self, String> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        self.file = Some(file);
        Ok(self)
    }
    
    pub fn debug(&mut self, msg: &str) {
        if self.level <= LogLevel::Debug {
            self.log("DEBUG", msg);
        }
    }
    
    pub fn info(&mut self, msg: &str) {
        if self.level <= LogLevel::Info {
            self.log("INFO", msg);
        }
    }
    
    pub fn warn(&mut self, msg: &str) {
        if self.level <= LogLevel::Warn {
            self.log("WARN", msg);
        }
    }
    
    pub fn error(&mut self, msg: &str) {
        if self.level <= LogLevel::Error {
            self.log("ERROR", msg);
        }
    }
    
    fn log(&mut self, level: &str, msg: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let log_line = format!("[{}] {}: {}", timestamp, level, msg);
        
        println!("{}", log_line);
        
        if let Some(ref mut file) = self.file {
            let _ = writeln!(file, "{}", log_line);
        }
    }
}
