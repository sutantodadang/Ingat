# Ingat Setup Guide

**Complete installation and configuration guide for all users**

This guide covers everything from installation to IDE integration, including the new multi-client remote mode that allows simultaneous UI and IDE usage without database conflicts.

---

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Multi-Client Setup (Recommended)](#multi-client-setup-recommended)
4. [IDE Integration](#ide-integration)
5. [Configuration](#configuration)
6. [Troubleshooting](#troubleshooting)
7. [Advanced Usage](#advanced-usage)

---

## Installation

### Prerequisites

**Required:**
- [Node.js](https://nodejs.org/) v18+ or [Bun](https://bun.sh/) (recommended)
- [Rust](https://rustup.rs/) latest stable toolchain
- [Git](https://git-scm.com/)

**Platform-specific:**
- **Windows:** [Microsoft Visual C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- **macOS:** Xcode Command Line Tools (`xcode-select --install`)
- **Linux:** Build essentials (`sudo apt install build-essential libwebkit2gtk-4.0-dev`)

### Download and Install

#### Option 1: From Source (Development)

```bash
# Clone the repository
git clone https://github.com/sutantodadang/Ingat.git
cd Ingat

# Install frontend dependencies
bun install  # or: npm install

# Build all MCP binaries
cd src-tauri
cargo build --release --bin mcp-stdio --features mcp-server
cargo build --release --bin mcp-bridge --features mcp-server
cargo build --release --bin mcp-service --features mcp-server,tauri-plugin
cd ..
```

#### Option 2: Pre-built Installer (Coming Soon)

Download the installer for your platform from the [Releases](https://github.com/sutantodadang/Ingat/releases) page.

**Windows:** `ingat-setup.exe`  
**macOS:** `ingat.dmg`  
**Linux:** `ingat.AppImage` or `ingat.deb`

The installer includes:
- Tauri desktop application
- All MCP server binaries (`mcp-stdio`, `mcp-bridge`, `mcp-service`)
- Helper scripts
- Documentation

---

## Quick Start

### Single Client Mode (Simplest)

**Use this if:** You only want to use ONE client at a time (either UI OR an IDE, but not both).

```bash
# Just run the UI
bun run dev

# Or use an IDE alone (configure it first - see IDE Integration section)
```

This mode opens the database directly. It's simple but **doesn't support simultaneous access**.

### Multi-Client Mode (Recommended)

**Use this if:** You want to use the UI AND your IDE(s) at the same time.

**Windows PowerShell:**
```powershell
# Automatic setup
.\start-with-service.ps1
```

**Manual steps:**
```bash
# 1. Start the service (keep this terminal open)
.\src-tauri\target\release\mcp-service.exe

# 2. In a new terminal, start the UI
bun run dev

# 3. Connect your IDE(s) - see IDE Integration section
```

**What happens:**
- `mcp-service` holds the database lock
- UI and IDEs connect via HTTP (remote mode)
- No database conflicts!

---

## Multi-Client Setup (Recommended)

### Architecture Overview

```
┌─────────────┐
│  Tauri UI   │────┐
└─────────────┘    │
                   ├───► mcp-service ───► Local Database
┌─────────────┐    │     (Single lock)     (sled)
│  VS Code    │────┤
└─────────────┘    │
                   │
┌─────────────┐    │
│    Zed      │────┘
└─────────────┘
```

### Step 1: Build the Service

If you haven't already:

```bash
cd src-tauri
cargo build --release --bin mcp-service --features mcp-server,tauri-plugin
cd ..
```

Binary location: `src-tauri/target/release/mcp-service.exe` (Windows) or `mcp-service` (Unix)

### Step 2: Start the Service

**Windows:**
```powershell
.\src-tauri\target\release\mcp-service.exe
```

**macOS/Linux:**
```bash
./src-tauri/target/release/mcp-service
```

**Expected output:**
```
Starting Ingat Backend Service v0.1.0
Initializing application environment...
Data directory: [your data path]
Application initialized successfully
🚀 Ingat Backend Service listening on http://127.0.0.1:3200
📊 Health check: http://127.0.0.1:3200/health
🔌 MCP SSE endpoint: http://127.0.0.1:3200/sse
💾 REST API: http://127.0.0.1:3200/api/*
```

**Verify it's running:**
```bash
curl http://localhost:3200/health
# Expected: {"status":"healthy","service":"ingat-backend"}
```

### Step 3: Start the UI

**In a NEW terminal:**

```bash
bun run dev
```

**Look for these logs:**
```
[ingat] Checking for mcp-service at 127.0.0.1:3200...
[ingat] ✓ Detected running mcp-service at 127.0.0.1:3200
[ingat] ✓ Using REMOTE MODE - all operations will proxy to the service
[ingat] ✓ No local database lock will be acquired
```

✅ **Success!** The UI is now connected to the service.

### Step 4: Connect IDEs

Now you can connect any number of IDEs. They will all automatically detect the running service and use remote mode.

See [IDE Integration](#ide-integration) section below.

---

## IDE Integration

### Supported IDEs

| IDE | Transport | Configuration File | Details |
|-----|-----------|-------------------|---------|
| VS Code | stdio | `.vscode/settings.json` | [Link](#vs-code) |
| Cursor | stdio | `.cursor/mcp.json` | [Link](#cursor) |
| Windsurf | stdio | `.windsurf/mcp.json` | [Link](#windsurf) |
| Sublime Text | stdio | Codeium settings | [Link](#sublime-text) |
| Zed | SSE | `settings.json` | [Link](#zed) |
| Claude Desktop | SSE | `claude_desktop_config.json` | [Link](#claude-desktop) |

---

### VS Code

**Requirements:** [MCP extension](https://marketplace.visualstudio.com/items?itemName=modelcontextprotocol.mcp)

**Configuration:**

1. Open your project in VS Code
2. Create/edit `.vscode/settings.json`:

```json
{
  "mcp.servers": {
    "ingat": {
      "command": "C:\\path\\to\\ingat\\src-tauri\\target\\release\\mcp-stdio.exe",
      "args": []
    }
  }
}
```

**On macOS/Linux:**
```json
{
  "mcp.servers": {
    "ingat": {
      "command": "/path/to/ingat/src-tauri/target/release/mcp-stdio",
      "args": []
    }
  }
}
```

3. Restart VS Code or reload window
4. Open the MCP panel to verify connection

**Logs:** Check Output panel > Select "MCP" from dropdown

---

### Cursor

**Requirements:** Built-in MCP support (v0.40+)

**Configuration:**

1. Create `.cursor/mcp.json` in your project:

```json
{
  "mcpServers": {
    "ingat": {
      "command": "C:\\path\\to\\ingat\\src-tauri\\target\\release\\mcp-stdio.exe",
      "args": []
    }
  }
}
```

**On macOS/Linux:**
```json
{
  "mcpServers": {
    "ingat": {
      "command": "/path/to/ingat/src-tauri/target/release/mcp-stdio",
      "args": []
    }
  }
}
```

2. Restart Cursor
3. Verify connection in MCP panel

---

### Windsurf

**Requirements:** Built-in MCP support

**Configuration:**

1. Create `.windsurf/mcp.json` in your project:

```json
{
  "mcpServers": {
    "ingat": {
      "command": "C:\\path\\to\\ingat\\src-tauri\\target\\release\\mcp-stdio.exe",
      "args": []
    }
  }
}
```

2. Restart Windsurf

---

### Sublime Text

**Requirements:** [Codeium plugin](https://codeium.com/sublime) with MCP support

**Configuration:**

1. Open Sublime Text
2. Go to Preferences > Package Settings > Codeium > Settings
3. Add to your settings:

```json
{
  "mcp_servers": {
    "ingat": {
      "command": "C:\\path\\to\\ingat\\src-tauri\\target\\release\\mcp-stdio.exe",
      "args": []
    }
  }
}
```

4. Restart Sublime Text

---

### Zed

**Requirements:** Zed editor with MCP support

**Configuration:**

1. Open Zed settings: `Cmd/Ctrl + ,`
2. Add to your `settings.json`:

```json
{
  "context_servers": {
    "ingat": {
      "settings": {
        "url": "http://localhost:3200"
      }
    }
  }
}
```

**Note:** Zed uses SSE transport and connects to `mcp-service` directly. Make sure the service is running first!

---

### Claude Desktop

**Requirements:** Claude Desktop app with MCP support

**Configuration:**

**Windows:** Edit `%APPDATA%\Claude\claude_desktop_config.json`  
**macOS:** Edit `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "ingat": {
      "url": "http://localhost:3200"
    }
  }
}
```

Restart Claude Desktop.

---

## Configuration

### Environment Variables

You can customize Ingat using environment variables:

```bash
# Service host and port
export INGAT_SERVICE_HOST="127.0.0.1"  # Default: 127.0.0.1
export INGAT_SERVICE_PORT="3200"        # Default: 3200

# Custom data directory
export INGAT_DATA_DIR="/custom/path"

# Logging level
export INGAT_LOG="info"  # Options: trace, debug, info, warn, error

# MCP server settings (for mcp-bridge)
export INGAT_MCP_BIND_ADDR="127.0.0.1:5210"
export INGAT_MCP_SSE_PATH="/sse"
export INGAT_MCP_POST_PATH="/message"
```

**Windows PowerShell:**
```powershell
$env:INGAT_SERVICE_PORT = "3200"
$env:INGAT_LOG = "debug"
```

### Data Storage Locations

**Default locations:**
- **Windows:** `%APPDATA%\ingat\Ingat\data`
- **macOS:** `~/Library/Application Support/ingat/Ingat/data`
- **Linux:** `~/.config/ingat/Ingat/data`

**Contents:**
- `store/` - Database files (sled)
- `config.json` - User configuration
- `embeddings/` - Cached embeddings (if using FastEmbed)

### Configuration File

Edit `config.json` in your data directory:

```json
{
  "embedding": {
    "backend": "simple",
    "model": "ingat/simple-hash",
    "dimensions": 384
  },
  "search": {
    "default_limit": 8,
    "max_results": 50
  }
}
```

**Embedding backends:**
- `"simple"` - Lightweight deterministic hash (default)
- `"fastembed"` - High-quality semantic embeddings (requires FastEmbed feature)

---

## Troubleshooting

### Database Lock Errors

**Symptom:**
```
storage failure: failed to open sled db: IO error: could not acquire lock
```

**Cause:** Multiple processes trying to open the database directly.

**Solution:**

1. **Stop everything:**
   ```powershell
   # Windows
   Get-Process | Where-Object {$_.ProcessName -match "ingat|mcp"} | Stop-Process -Force
   
   # macOS/Linux
   pkill -f "ingat|mcp"
   ```

2. **Use multi-client mode:**
   - Start `mcp-service` first
   - Then start UI and IDEs
   - See [Multi-Client Setup](#multi-client-setup-recommended)

**Quick fix:** `.\start-with-service.ps1` (Windows)

---

### Service Won't Start

**Symptom:** Service exits immediately or shows "could not acquire lock"

**Cause:** UI or another process has the database open

**Solution:**

1. **Check for running processes:**
   ```powershell
   # Windows
   Get-Process | Where-Object {$_.ProcessName -match "ingat|mcp"}
   
   # macOS/Linux
   ps aux | grep -i ingat
   ```

2. **Kill all Ingat processes:**
   ```powershell
   # Windows
   Get-Process -Name ingat,mcp-* | Stop-Process -Force
   
   # macOS/Linux
   killall ingat mcp-stdio mcp-bridge mcp-service
   ```

3. **Start service first, THEN UI**

---

### IDE Connection Fails

**Symptom:** VS Code/Cursor shows "Connection state: Error"

**Causes & Solutions:**

1. **Service not running**
   ```bash
   # Check service health
   curl http://localhost:3200/health
   
   # If fails, start the service
   .\src-tauri\target\release\mcp-service.exe
   ```

2. **Wrong binary path**
   - Verify path in IDE config is correct
   - Use absolute paths: `C:\full\path\to\mcp-stdio.exe`

3. **Binary not built**
   ```bash
   # Rebuild
   cd src-tauri
   cargo build --release --bin mcp-stdio --features mcp-server
   ```

4. **Check IDE logs**
   - VS Code: Output panel > Select "MCP"
   - Look for error messages

---

### UI Uses Local Mode Instead of Remote

**Symptom:** UI logs show "using local database mode"

**Cause:** Service isn't running or not accessible

**Solution:**

1. **Verify service is running:**
   ```bash
   curl http://localhost:3200/health
   ```

2. **Check for firewall blocking port 3200**

3. **Start service manually:**
   ```bash
   .\src-tauri\target\release\mcp-service.exe
   ```

4. **Restart UI** - it will detect the service

---

### Port Already in Use

**Symptom:** "Address already in use: 127.0.0.1:3200"

**Cause:** Another process is using port 3200

**Solution:**

1. **Find what's using the port:**
   ```powershell
   # Windows
   netstat -ano | findstr :3200
   
   # macOS/Linux
   lsof -i :3200
   ```

2. **Kill the process or use a different port:**
   ```bash
   export INGAT_SERVICE_PORT=3201
   .\src-tauri\target\release\mcp-service.exe
   ```

---

### Binary Not Found

**Symptom:** IDE can't find `mcp-stdio.exe`

**Solution:**

1. **Check file exists:**
   ```bash
   ls src-tauri/target/release/mcp-stdio*
   ```

2. **Build it:**
   ```bash
   cd src-tauri
   cargo build --release --bin mcp-stdio --features mcp-server
   ```

3. **Update IDE config with correct path**

---

## Advanced Usage

### Remote Service Access

**⚠️ Security Warning:** Only expose the service over a network with proper security measures!

To allow remote connections:

1. **Bind to all interfaces:**
   ```bash
   export INGAT_SERVICE_HOST="0.0.0.0"
   .\src-tauri\target\release\mcp-service.exe
   ```

2. **Set up reverse proxy with TLS (recommended):**

   **nginx example:**
   ```nginx
   server {
       listen 443 ssl;
       server_name ingat.example.com;
       
       ssl_certificate /path/to/cert.pem;
       ssl_certificate_key /path/to/key.pem;
       
       location / {
           proxy_pass http://localhost:3200;
           proxy_http_version 1.1;
           proxy_set_header Upgrade $http_upgrade;
           proxy_set_header Connection "upgrade";
           proxy_set_header Host $host;
       }
   }
   ```

3. **Configure clients to use remote URL:**
   ```bash
   export INGAT_SERVICE_HOST="ingat.example.com"
   export INGAT_SERVICE_PORT="443"
   ```

**TODO:** Future versions will include authentication and API keys.

---

### Using FastEmbed

For higher quality semantic embeddings:

1. **Build with FastEmbed:**
   ```bash
   cd src-tauri
   cargo build --release --features fastembed-engine,mcp-server,tauri-plugin
   ```

2. **Update config:**
   Edit your data directory's `config.json`:
   ```json
   {
     "embedding": {
       "backend": "fastembed",
       "model": "BAAI/bge-small-en-v1.5",
       "dimensions": 384
     }
   }
   ```

3. **Restart service/UI**

**Note:** FastEmbed downloads ONNX models (~100MB) on first run.

---

### Running as System Service

**Linux (systemd):**

Create `/etc/systemd/system/ingat-service.service`:

```ini
[Unit]
Description=Ingat MCP Service
After=network.target

[Service]
Type=simple
User=youruser
ExecStart=/path/to/mcp-service
Restart=on-failure
Environment="INGAT_LOG=info"

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable ingat-service
sudo systemctl start ingat-service
sudo systemctl status ingat-service
```

**macOS (launchd):**

Create `~/Library/LaunchAgents/com.ingat.service.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.ingat.service</string>
    <key>ProgramArguments</key>
    <array>
        <string>/path/to/mcp-service</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

Load:
```bash
launchctl load ~/Library/LaunchAgents/com.ingat.service.plist
```

**Windows (NSSM):**

1. Download [NSSM](https://nssm.cc/)
2. Install service:
   ```powershell
   nssm install ingat-service "C:\path\to\mcp-service.exe"
   nssm start ingat-service
   ```

---

### Multiple Instances

Run multiple isolated instances with different data directories:

```bash
# Instance 1 (default)
INGAT_DATA_DIR=~/.ingat/personal INGAT_SERVICE_PORT=3200 ./mcp-service &

# Instance 2 (work)
INGAT_DATA_DIR=~/.ingat/work INGAT_SERVICE_PORT=3201 ./mcp-service &
```

Configure clients to point to specific ports.

---

## Helper Scripts

### Windows PowerShell

**Start everything:**
```powershell
.\start-with-service.ps1
```

**Check service status:**
```powershell
.\scripts\check-service.ps1
```

**Stop service:**
```powershell
.\scripts\stop-service.ps1
```

### Unix/macOS

**Check service:**
```bash
./scripts/check-service.sh
```

**Stop service:**
```bash
./scripts/stop-service.sh
```

---

## Getting Help

### Documentation

- **This Guide:** Complete setup instructions
- **[START_HERE.md](./START_HERE.md)** - Quick troubleshooting
- **[QUICK_FIX.md](./QUICK_FIX.md)** - Fix database lock issues
- **[docs/REMOTE_MODE.md](./docs/REMOTE_MODE.md)** - Technical details on remote mode
- **[MCP_INTEGRATION.md](./MCP_INTEGRATION.md)** - MCP protocol documentation
- **[IDE_MCP_SETUP.md](./IDE_MCP_SETUP.md)** - IDE-specific guides

### Support Channels

- 🐛 **Bug Reports:** [GitHub Issues](https://github.com/sutantodadang/Ingat/issues)
- 💬 **Discussions:** [GitHub Discussions](https://github.com/sutantodadang/Ingat/discussions)
- 📖 **Wiki:** [GitHub Wiki](https://github.com/sutantodadang/Ingat/wiki)

### Common Questions

**Q: Can I use the UI without the service?**  
A: Yes! Just run `bun run dev`. The UI will use local mode (direct DB access). However, you can't connect IDEs at the same time in local mode.

**Q: Do I always need to start the service manually?**  
A: No. The UI tries to auto-start the service. But if you prefer manual control, start the service first, then the UI will detect it automatically.

**Q: Can multiple people use the same service?**  
A: Yes! Run `mcp-service` on a shared server and have team members connect their IDEs to it. See [Remote Service Access](#remote-service-access).

**Q: Which embedding backend should I use?**  
A: Start with `simple` (default). It's fast and works offline. Upgrade to `fastembed` if you need better semantic search quality.

**Q: How do I backup my data?**  
A: Copy your entire data directory (see [Data Storage Locations](#data-storage-locations)). The database is in the `store/` subdirectory.

---

## Next Steps

1. ✅ **Install Ingat** - Follow [Installation](#installation)
2. ✅ **Run the UI** - Use [Quick Start](#quick-start)
3. ✅ **Connect your IDE** - See [IDE Integration](#ide-integration)
4. ✅ **Enable multi-client** - Set up [Multi-Client Mode](#multi-client-setup-recommended)
5. 📖 **Read the docs** - Explore [Documentation](#getting-help)

---

## Summary

✅ **Single Client:** Just run `bun run dev` (simple, but one client at a time)  
✅ **Multi-Client:** Use `mcp-service` + UI + IDEs (recommended, no conflicts)  
✅ **Automatic Detection:** Everything detects the service automatically  
✅ **Zero Config:** Default settings work for 99% of users  

**Get started now with:** `.\start-with-service.ps1` (Windows) 🚀

---

*Last updated: 2024-11-15*