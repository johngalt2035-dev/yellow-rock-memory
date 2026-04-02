# Installation Guide

Install yellow-rock-memory on any platform. Pick your operating system below.

> **No programming experience?** Skip to [AI-Assisted Installation](AI-INSTALL.md) — let ChatGPT, Claude, or your AI assistant do it for you.

> **Documentation Map**: You are here → **INSTALL.md**. Next → [SETUP.md](SETUP.md) (configure contacts). Then → [AI-INSTALL.md](AI-INSTALL.md) (connect to AI + test). Also see → [Yellow Rock Protocol](https://github.com/johngalt2035-dev/yellow-rock-protocol) (communication templates).

---

## What You're Installing

Yellow Rock Memory is a program that stores and organizes messages for the Yellow Rock communication system. It runs in the background on your computer and provides a database that your AI assistant can use.

**You need**: ~3 GB free disk space, 4 GB RAM, and an internet connection for the initial download.

**Build time**: The first build takes **5-20 minutes** depending on your computer speed. This is normal — it's compiling the program. You'll see lots of text scrolling. Don't close the terminal.

---

## macOS

### Prerequisites

Open **Terminal** (Applications > Utilities > Terminal) and run these commands one at a time:

```bash
# 1. Install Xcode Command Line Tools (required for building software)
xcode-select --install
# A popup will appear. Click "Install" and wait for it to finish.

# 2. Install Rust (the programming language this is written in)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Press Enter to accept the default installation.
# When done, run this to activate it:
source "$HOME/.cargo/env"

# 3. Verify both are installed:
git --version    # Should show: git version 2.x.x
rustc --version  # Should show: rustc 1.x.x
```

### Build & Install

```bash
# 4. Download the source code
git clone https://github.com/johngalt2035-dev/yellow-rock-memory.git
cd yellow-rock-memory

# 5. Build it (this takes 5-20 minutes the first time — that's normal!)
cargo build --release

# 6. Install the program
#    On Apple Silicon (M1/M2/M3/M4 Macs):
sudo cp target/release/yellow-rock-memory /opt/homebrew/bin/
#    On Intel Macs:
#    sudo cp target/release/yellow-rock-memory /usr/local/bin/
# (sudo will ask for your password — type it and press Enter, nothing will appear as you type)

# 7. Verify it works
yellow-rock-memory --help
# You should see "Yellow Rock memory system" and a list of commands
```

### Start the Memory System

```bash
# 8. Create the database folder
mkdir -p ~/.yellow-rock

# 9. Start it
yellow-rock-memory --db ~/.yellow-rock/memory.db serve --port 9077

# 10. Open a NEW terminal window and test:
curl http://localhost:9077/api/v1/health
# Should show: {"status":"ok","service":"yellow-rock-memory"}
```

### Run Automatically at Login (Optional)

Create `~/Library/LaunchAgents/com.yellow-rock.memory.plist` — see [auto-start guide](https://github.com/johngalt2035-dev/yellow-rock-memory/blob/main/SETUP.md#auto-start).

Or simply add to your `~/.zshrc`:
```bash
echo 'yellow-rock-memory --db ~/.yellow-rock/memory.db serve --port 9077 &' >> ~/.zshrc
```

---

## Windows

### Prerequisites

**You need TWO things before building:**

1. **Visual Studio C++ Build Tools** (required by Rust on Windows):
   - Go to [visualstudio.microsoft.com/visual-cpp-build-tools/](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   - Download and run the installer
   - Check **"Desktop development with C++"** and click Install
   - This is ~2 GB and takes several minutes

2. **Rust**:
   - Go to [rustup.rs](https://rustup.rs/)
   - Click **"DOWNLOAD RUSTUP-INIT.EXE (64-BIT)"**
   - Run the downloaded file and press Enter to accept defaults
   - **Restart your computer** after installing

3. **Git**:
   - Go to [git-scm.com](https://git-scm.com/)
   - Download and install with default settings

### Build & Install

Open **PowerShell** (search for "PowerShell" in Start menu) and run:

```powershell
# 1. Verify prerequisites
git --version    # Should show: git version 2.x.x
rustc --version  # Should show: rustc 1.x.x

# 2. Download the source code
git clone https://github.com/johngalt2035-dev/yellow-rock-memory.git
cd yellow-rock-memory

# 3. Build it (5-20 minutes the first time — that's normal!)
cargo build --release

# 4. Create a bin folder and copy the program there
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\bin"
Copy-Item "target\release\yellow-rock-memory.exe" "$env:USERPROFILE\bin\"

# 5. Add to PATH (so you can run it from anywhere)
$oldPath = [Environment]::GetEnvironmentVariable("Path", "User")
$newPath = "$oldPath;$env:USERPROFILE\bin"
[Environment]::SetEnvironmentVariable("Path", $newPath, "User")

# 6. RESTART PowerShell (close and reopen it), then verify:
yellow-rock-memory --help
```

> **Windows Firewall**: When you start the daemon, Windows may show a firewall popup. Click **"Allow access"** — the program only listens on your local machine.

### Start the Memory System

```powershell
# 7. Create the database folder
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.yellow-rock"

# 8. Start it
yellow-rock-memory --db "$env:USERPROFILE\.yellow-rock\memory.db" serve --port 9077

# 9. Open a NEW PowerShell window and test:
Invoke-WebRequest http://localhost:9077/api/v1/health | Select-Object -ExpandProperty Content
# Should show: {"status":"ok","service":"yellow-rock-memory"}
```

### Alternative: Windows Subsystem for Linux (WSL)

If you're comfortable with Linux, WSL is easier:
```powershell
# Install WSL (in Admin PowerShell)
wsl --install

# Then follow the Ubuntu instructions below inside WSL
```

### Run Automatically at Login (Optional)

```powershell
# Create a startup shortcut
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\GreyRockMemory.lnk")
$Shortcut.TargetPath = "$env:USERPROFILE\bin\yellow-rock-memory.exe"
$Shortcut.Arguments = "--db $env:USERPROFILE\.yellow-rock\memory.db serve --port 9077"
$Shortcut.Save()
```

---

## Ubuntu / Debian

### Prerequisites & Build

Open a terminal and run:

```bash
# 1. Install prerequisites
sudo apt update
sudo apt install -y build-essential git curl

# 2. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Press Enter to accept defaults, then:
source "$HOME/.cargo/env"

# 3. Verify
git --version && rustc --version

# 4. Download and build (5-20 minutes first time!)
git clone https://github.com/johngalt2035-dev/yellow-rock-memory.git
cd yellow-rock-memory
cargo build --release

# 5. Install
sudo cp target/release/yellow-rock-memory /usr/local/bin/

# 6. Verify
yellow-rock-memory --help
```

### Start the Memory System

```bash
# 7. Create database folder
mkdir -p ~/.yellow-rock

# 8. Start
yellow-rock-memory --db ~/.yellow-rock/memory.db serve --port 9077

# 9. Test (in another terminal)
curl http://localhost:9077/api/v1/health
# Should show: {"status":"ok","service":"yellow-rock-memory"}
```

### Run Automatically (systemd)

```bash
# Create service file
sudo tee /etc/systemd/system/yellow-rock-memory.service << 'EOF'
[Unit]
Description=Yellow Rock Memory System
After=network.target

[Service]
Type=simple
User=YOUR_USERNAME
ExecStart=/usr/local/bin/yellow-rock-memory --db /home/YOUR_USERNAME/.yellow-rock/memory.db serve --port 9077
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Replace YOUR_USERNAME with your actual username:
sudo sed -i "s/YOUR_USERNAME/$USER/g" /etc/systemd/system/yellow-rock-memory.service

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable yellow-rock-memory
sudo systemctl start yellow-rock-memory
sudo systemctl status yellow-rock-memory
```

---

## Fedora / RHEL / CentOS

Same as Ubuntu except for the prerequisites step:

```bash
# 1. Install prerequisites (Fedora uses dnf instead of apt)
sudo dnf install -y gcc git curl

# Rest is identical — follow Ubuntu steps 2-9 above
```

For the systemd service, same file as Ubuntu. Replace `YOUR_USERNAME` with your username.

---

## After Installation (All Platforms)

### Set Your API Key

Your AI assistant needs an API key. Set it as an environment variable:

**macOS / Linux** — add to your shell config:
```bash
# For OpenAI:
echo 'export OPENAI_API_KEY="sk-your-key-here"' >> ~/.zshrc  # macOS
echo 'export OPENAI_API_KEY="sk-your-key-here"' >> ~/.bashrc  # Linux
source ~/.zshrc  # or source ~/.bashrc

# For other providers, use:
# ANTHROPIC_API_KEY, XAI_API_KEY, GOOGLE_API_KEY
```

**Windows** — in PowerShell:
```powershell
# For OpenAI:
[Environment]::SetEnvironmentVariable("OPENAI_API_KEY", "sk-your-key-here", "User")
# Restart PowerShell after setting.

# For other providers: ANTHROPIC_API_KEY, XAI_API_KEY, GOOGLE_API_KEY
```

### Train with Background Knowledge

Create a file called `background.json` (use any text editor — Notepad, TextEdit, nano):

```json
[
  {
    "title": "Contact schedule",
    "content": "Contact works Mon-Fri 9am-5pm."
  },
  {
    "title": "Conference call",
    "content": "Standing call at 3:15 PM with external parties."
  }
]
```

> **Tip**: JSON requires exact syntax — make sure every `"` is matched and every `{` has a `}`. If you get an error, try an [online JSON validator](https://jsonlint.com/).

Import it:
```bash
yellow-rock-memory --db ~/.yellow-rock/memory.db train background.json
# Should show: Imported 2 memories into namespace "yellow-rock" (skipped 0)
```

**You can also use Markdown** — simpler for longer content:
```bash
yellow-rock-memory --db ~/.yellow-rock/memory.db train background.md
```

Where `background.md` looks like:
```markdown
# Contact Schedule
Contact works Mon-Fri 9am-5pm. Commute is 25 minutes.

# Conference Call
Standing call at 3:15 PM with external parties.
```

### YAML Format (simpler for training data)

Save as `background.yaml`:
```yaml
- title: Contact schedule
  content: Contact works Mon-Fri 9am-5pm.

- title: Meeting cadence
  content: Standing call Tuesdays at 3pm with external parties.

- title: Personal communication style
  content: Uses contractions, short sentences, direct tone.
  tags: [style, personal]
  priority: 8
```

Import: `yellow-rock-memory --db ~/.yellow-rock/memory.db train background.yaml`

YAML is often easier to write by hand — no brackets, no quoted keys, no trailing commas to worry about.

### Verify Everything Works

```bash
# Check health
curl http://localhost:9077/api/v1/health
# ✅ {"status":"ok","service":"yellow-rock-memory"}

# Check stats
yellow-rock-memory --db ~/.yellow-rock/memory.db stats
# ✅ Shows memory count, tiers, namespaces

# Test recall
yellow-rock-memory --db ~/.yellow-rock/memory.db recall "conference call"
# ✅ Returns the training data you imported
```

### What Training Data to Include

Here are the most useful types of background knowledge:

| Category | Examples |
|---|---|
| **Schedules** | Work hours, meeting times, deadlines, activities |
| **Contact info** | Names, roles, phone numbers |
| **Agreements** | Contractual terms, financial arrangements, operating rules |
| **Patterns** | Known escalation triggers, communication patterns |
| **Key contacts** | Attorneys, accountants, professional contacts |
| **Boundaries** | What topics to defer, financial limits, legal items |
| **Communication Styles** | Personal phrases, executive phrases, tone preferences |

### Communication Style Training

Train the system to match your preferred tone per contact.

**Personal style** (warm, conversational):
```json
[
  {
    "title": "Personal style phrases",
    "content": "Use casual greetings like 'Hey' or 'Hi there'. Sign off with 'Talk soon' or 'Take care'. Use contractions freely. Mirror the contact's energy level.",
    "tags": ["style", "personal"],
    "priority": 8
  }
]
```

**Executive style** (formal, factual):
```json
[
  {
    "title": "Executive style phrases",
    "content": "Use neutral greetings like 'Good morning' or none at all. Keep responses under two sentences. No emotional language. Sign off with 'Regards' or omit entirely.",
    "tags": ["style", "executive"],
    "priority": 8
  }
]
```

Import style training the same way:
```bash
yellow-rock-memory --db ~/.yellow-rock/memory.db train style-personal.json
yellow-rock-memory --db ~/.yellow-rock/memory.db train style-executive.json
```

---

## Next Steps

1. **Configure your contacts** → [SETUP.md](SETUP.md)
2. **Connect to your AI assistant** → [AI-INSTALL.md](AI-INSTALL.md)
3. **Get the communication protocol** → [Yellow Rock Protocol](https://github.com/johngalt2035-dev/yellow-rock-protocol)

---

## Troubleshooting

**"command not found: yellow-rock-memory"**
- Make sure the binary is in your PATH
- macOS: `ls /opt/homebrew/bin/yellow-rock-memory` or `ls /usr/local/bin/yellow-rock-memory`
- Windows: `ls $env:USERPROFILE\bin\yellow-rock-memory.exe`
- Linux: `ls /usr/local/bin/yellow-rock-memory`

**"failed to open database"**
- Create the directory: `mkdir -p ~/.yellow-rock` (Mac/Linux) or `New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.yellow-rock"` (Windows)

**"port 9077 already in use"**
- Another instance is running. Kill it:
  - Mac/Linux: `pkill yellow-rock-memory`
  - Windows: `Stop-Process -Name yellow-rock-memory`

**Build fails with "linker cc not found"**
- macOS: `xcode-select --install`
- Ubuntu: `sudo apt install build-essential`
- Fedora: `sudo dnf install gcc`
- **Windows: Install Visual Studio C++ Build Tools** (see Windows Prerequisites above)

**Build fails on Windows with "link.exe not found"**
- You need Visual Studio C++ Build Tools. Download from [visualstudio.microsoft.com](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- Check "Desktop development with C++" during installation

**JSON training file gives parse error**
- Check your JSON at [jsonlint.com](https://jsonlint.com/)
- Common mistakes: missing comma between entries, missing closing `]`, mismatched quotes

**"Permission denied" on macOS**
- Use `sudo` before the copy command
- If `/usr/local/bin` doesn't exist: `sudo mkdir -p /usr/local/bin`

**Windows Firewall blocks the daemon**
- Click "Allow access" when the popup appears
- Or manually: Windows Security > Firewall > Allow an app > yellow-rock-memory

---

*Yellow Rock Memory works with [Yellow Rock Protocol](https://github.com/johngalt2035-dev/yellow-rock-protocol). Both are built upon [OpenClaw](https://openclaw.ai). See [LICENSE](LICENSE) and [LEGAL_DISCLAIMER.md](LEGAL_DISCLAIMER.md) for terms.*
