use std::io::Write;
use std::os::unix::net::UnixStream;
use serde_json::json;

/// Neovim RPC client for integration.
/// Sends commands over the Neovim Unix socket using msgpack-rpc.
pub struct NeovimClient {
    socket_path: String,
}

impl NeovimClient {
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.to_string(),
        }
    }

    /// Try to discover a running Neovim instance socket
    pub fn discover() -> Option<String> {
        // Check common locations
        if let Ok(nvim_listen) = std::env::var("NVIM_LISTEN_ADDRESS") {
            return Some(nvim_listen);
        }
        if let Ok(nvim) = std::env::var("NVIM") {
            return Some(nvim);
        }

        // Check XDG runtime dir
        if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
            let nvim_dir = std::path::Path::new(&runtime).join("nvim");
            if nvim_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&nvim_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let socket = path.join("0");
                            if socket.exists() {
                                return Some(socket.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Send a code block to Neovim in a new scratch buffer
    pub fn send_to_buffer(&self, content: &str, filetype: &str) -> anyhow::Result<()> {
        let mut stream = UnixStream::connect(&self.socket_path)?;

        // Use nvim_exec2 to create a scratch buffer and insert content
        let commands = format!(
            "enew | setlocal buftype=nofile bufhidden=wipe noswapfile | set filetype={} | normal! i{}",
            filetype,
            content.replace('\\', "\\\\").replace('"', "\\\"")
        );

        let request = json!([0, 1, "nvim_exec2", [commands, {}]]);
        let data = serde_json::to_vec(&request)?;
        stream.write_all(&data)?;
        stream.flush()?;

        Ok(())
    }

    /// Open a file in Neovim
    pub fn open_file(&self, path: &str) -> anyhow::Result<()> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        let request = json!([0, 1, "nvim_command", [format!("edit {path}")]]);
        let data = serde_json::to_vec(&request)?;
        stream.write_all(&data)?;
        stream.flush()?;
        Ok(())
    }

    /// Get current buffer content from Neovim
    pub fn get_current_buffer(&self) -> anyhow::Result<String> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        let request = json!([0, 1, "nvim_exec2", ["echo join(getline(1, '$'), \"\\n\")", {"output": true}]]);
        let data = serde_json::to_vec(&request)?;
        stream.write_all(&data)?;
        stream.flush()?;

        // Read response - simplified, real impl would properly parse msgpack
        let mut buf = [0u8; 65536];
        let n = std::io::Read::read(&mut stream, &mut buf)?;
        Ok(String::from_utf8_lossy(&buf[..n]).to_string())
    }

    pub fn is_connected(&self) -> bool {
        UnixStream::connect(&self.socket_path).is_ok()
    }
}
