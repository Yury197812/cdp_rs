use anyhow::Result;
use std::net::ToSocketAddrs;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub struct BrowserManager {
    binary: String,
    port: u16,
    child: Option<Child>,
    connected: bool,
}

impl BrowserManager {
    pub fn new() -> Self {
        Self {
            binary: "chrome".to_string(),
            port: 9222,
            child: None,
            connected: false,
        }
    }

    pub fn binary(mut self, binary: &str) -> Self {
        self.binary = binary.to_string();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub async fn launch(&mut self) -> Result<Self> {
        println!("[Browser] Launching {} on port {}", self.binary, self.port);
        
        // Check if port is in use
        if self.is_port_in_use().await {
            return Err(anyhow::anyhow!("Port {} already in use", self.port));
        }
        
        // Launch browser with CDP
        let child = Command::new(&self.binary)
            .args([
                &format!("--remote-debugging-port={}", self.port),
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-background-networking",
                "--disable-sync",
                "--disable-translate",
                "--disable-extensions",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to launch browser: {}", e))?;
        
        println!("[Browser] Process started (PID: {})", child.id());
        
        // Wait for CDP to be ready
        self.wait_for_cdp(Duration::from_secs(30)).await?;
        
        self.child = Some(child);
        self.connected = true;
        
        println!("[Browser] CDP connected on port {}", self.port);
        
        Ok(Self {
            binary: self.binary.clone(),
            port: self.port,
            child: self.child.take(),
            connected: true,
        })
    }

    async fn wait_for_cdp(&self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        let url = format!("http://127.0.0.1:{}/json/version", self.port);
        
        loop {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("CDP timeout after {:?}", timeout));
            }
            
            match reqwest::get(&url).await {
                Ok(resp) if resp.status().is_success() => {
                    println!("[Browser] CDP ready");
                    return Ok(());
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn is_port_in_use(&self) -> bool {
        format!("127.0.0.1:{}", self.port)
            .to_socket_addrs()
            .is_ok()
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            child.wait()?;
            println!("[Browser] Shut down");
        }
        self.connected = false;
        Ok(())
    }
}
