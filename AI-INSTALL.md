# AI-Assisted Installation

Let your AI assistant install Grey Rock for you. Copy-paste one prompt, and your AI does the rest.

> **Documentation Map**: [INSTALL.md](INSTALL.md) (manual install) → You are here → **AI-INSTALL.md** → [SETUP.md](SETUP.md) (configure contacts)

> **Which AI can install this?**
> - **OpenClaw**: Full automated setup (best option if you have OpenClaw) — [install OpenClaw](https://openclaw.ai)
> - **Claude Code**: Can clone, build, and configure directly on your machine
> - **ChatGPT / OpenAI**: Can generate commands for you to copy-paste (cannot execute directly)
> - **Any AI with terminal access**: Copy the prompts below and paste them into your AI

---

## Method 1: OpenClaw

If you have [OpenClaw](https://openclaw.ai) running, tell your agent:

```
Install the Grey Rock Executive Assistant communications system from these two repositories:
- https://github.com/johngalt2035-dev/grey-rock-protocol (communication templates)
- https://github.com/johngalt2035-dev/grey-rock-memory (memory + forensic archive)

Steps:
1. Clone both repos
2. Build grey-rock-memory: cargo build --release
3. Install the binary to /usr/local/bin/
4. Start the grey-rock-memory daemon on port 9077
5. Copy templates/openclaw/SOUL.md to my agent workspace
6. Replace {{USER_NAME}} with my name, {{CONTACT_NAME}} with the high-conflict counterparty's name
7. Replace {{CHANNEL}} with "signal" (or telegram/whatsapp)
8. Copy templates/openclaw/cron-jobs.json for automated morning message + daily digest
9. Configure my channel binding to route messages to the grey-rock agent
10. Test by sending a message to my own number (Note to Self)
```

### Adding Training Data via OpenClaw

```
Load the following background data into the Grey Rock Memory system:

grey-rock-memory --db ~/.grey-rock/memory.db train [FILE]

Create a training file with these facts:
- [Contact name] works [schedule]
- Conference call at [time] with [parties]
- [Any other facts the AI should know]

Import it and verify with: grey-rock-memory --db ~/.grey-rock/memory.db stats
```

### Setting Communication Styles via OpenClaw

```
Set the communication style for [Contact Name] to executive.
Set the communication style for [Contact Name] to personal.

Train personal style with these phrases:
- Casual greetings: "Hey", "Hi there"
- Sign-offs: "Talk soon", "Take care"
- Use contractions, mirror energy level

Train executive style with these phrases:
- Neutral greetings: "Good morning" or none
- Responses under two sentences, no emotional language
- Sign-offs: "Regards" or omit entirely
```

### Adding Contacts via OpenClaw

```
Add a new grey-rock contact:
- Name: [Their Name]
- Phone: [Their Number]
- Channel: signal (or telegram/whatsapp)
- Protocol: grey-rock (BIFF responses, logging, escalation tracking)

Update the openclaw.json allowFrom list to include their number.
Update the SOUL.md to include their contact_id for memory routing.
```

---

## Method 2: Claude Code

Open Claude Code in any project directory and say:

```
I want to set up the Grey Rock Executive Assistant communications system.
I am on [macOS / Windows / Ubuntu / Fedora].

1. Clone https://github.com/johngalt2035-dev/grey-rock-memory
2. Build it: cd grey-rock-memory && cargo build --release
   (Note: this takes 5-20 minutes the first time — that's normal)
3. Install the binary:
   - macOS Apple Silicon: cp target/release/grey-rock-memory /opt/homebrew/bin/
   - macOS Intel: cp target/release/grey-rock-memory /usr/local/bin/
   - Linux: cp target/release/grey-rock-memory /usr/local/bin/
   - Windows: copy target\release\grey-rock-memory.exe $env:USERPROFILE\bin\
   (Do NOT use sudo — if permission denied, use cp to ~/bin/ instead)
4. Create database: mkdir -p ~/.grey-rock
5. Start daemon: grey-rock-memory --db ~/.grey-rock/memory.db serve --port 9077
6. Verify: curl http://localhost:9077/api/v1/health

Then clone the protocol:
7. Clone https://github.com/johngalt2035-dev/grey-rock-protocol
8. Copy templates/claude-code/CLAUDE.md to my project root
9. Replace [CONTACT_NAME] with: [TYPE THE PERSON'S NAME HERE]

Now I want to train it with background data. Create a JSON file with these facts:
- [List your facts here, e.g.:]
- Contact works Mon-Fri 9-5
- Conference call at 3:15 PM
- Known escalation triggers: money, schedule changes

Import with: grey-rock-memory --db ~/.grey-rock/memory.db train [filename].json
Verify with: grey-rock-memory --db ~/.grey-rock/memory.db stats
```

> **Note**: Claude Code runs commands directly on your machine. If a `sudo` command fails (password required), Claude Code will suggest an alternative path that doesn't need sudo.

### Adding Training Data via Claude Code

```
Create a training file for grey-rock-memory with these facts about my situation:
- Meeting schedule: [describe]
- Contact's work hours: [describe]
- Key deadlines: [describe]
- Professional contacts: [list]

Format as JSON array of {title, content, tags, priority} objects.
Then import: grey-rock-memory --db ~/.grey-rock/memory.db train training.json
```

### Setting Communication Styles via Claude Code

```
Set the communication style for [Contact Name] to personal.
Set the communication style for [Contact Name] to executive.

Create training data for personal style:
- Casual tone, contractions, warm greetings ("Hey", "Hi there")
- Sign-offs like "Talk soon" or "Take care"

Create training data for executive style:
- Formal tone, no contractions, neutral greetings ("Good morning")
- Responses under two sentences, sign-off "Regards" or omit

Import both with: grey-rock-memory --db ~/.grey-rock/memory.db train style-[type].json
```

### Adding Contacts via Claude Code

```
I want to add a new high-conflict contact to Grey Rock:
- Name: [name]
- Their phone: [number]
- Platform: [signal/telegram/whatsapp]

Update my grey-rock-config.json to add them as a grey-rock protocol contact.
Generate appropriate training data for this contact and import it.
```

---

## Method 3: ChatGPT / OpenAI

> **Important**: ChatGPT runs in a cloud sandbox — it **cannot** install software on your computer. Instead, it will **generate the exact commands** for you to copy-paste into your terminal. This is still very helpful — it's like having an expert write your installation script.

In ChatGPT:

```
Help me install the Grey Rock Executive Assistant communications system.

Repositories:
- Protocol (templates): https://github.com/johngalt2035-dev/grey-rock-protocol
- Memory (Rust daemon): https://github.com/johngalt2035-dev/grey-rock-memory

I'm on [macOS/Windows/Ubuntu/Fedora].

Please:
1. Give me the exact commands to clone, build, and install grey-rock-memory
2. Show me how to start the daemon
3. Help me create a configuration file for my contacts
4. Help me create training data with my situation's background facts
5. Show me how to verify everything works
```

### Setting Communication Styles via ChatGPT

```
I need communication style training data for Grey Rock Memory.

Create two JSON training files:
1. Personal style: casual greetings, contractions, warm sign-offs ("Talk soon", "Take care")
2. Executive style: formal tone, under two sentences, neutral sign-offs ("Regards")

Format as: [{"title": "...", "content": "...", "tags": ["style", "personal"], "priority": 8}]
Give me the import commands for both files.
```

### Adding Training Data via ChatGPT

```
I need to create training data for Grey Rock Memory.
Here's my situation:
- [Describe your business context/conflict]
- [Key schedules]
- [Important facts]

Generate a background.json file in this format:
[{"title": "...", "content": "...", "tags": [...], "priority": N}, ...]

Then give me the command to import it.
```

---

## Testing Without Real Contacts

**IMPORTANT**: Never test with real contacts. Always test with yourself first.

### Quick Smoke Test (30 seconds)

Run this single command to verify everything works:

```bash
# Mac / Linux:
curl -s http://localhost:9077/api/v1/health && echo " ← If you see 'ok', the system is running!"

# Windows (PowerShell):
(Invoke-WebRequest http://localhost:9077/api/v1/health).Content
# Should show: {"status":"ok","service":"grey-rock-memory"}
```

If that works, the system is live. Now test the full pipeline:

### Signal — Test with Note to Self

```bash
# Send a test message to your own Note to Self
signal-cli -a +YOUR_NUMBER send --note-to-self -m "Test: Meeting confirmed at 5pm"

# Verify the system responds (if connected to an AI agent)
# Or manually test the memory system:
curl -X POST http://localhost:9077/api/v1/messages \
  -H 'Content-Type: application/json' \
  -d '{"sender":"test-contact","contact_id":"test","raw_content":"Meeting confirmed at 5pm","category":"LOGISTICS"}'

# Check it was archived
grey-rock-memory --db ~/.grey-rock/memory.db stats

# Check escalation (should be 1/ROUTINE for one message)
curl "http://localhost:9077/api/v1/escalation?contact_id=test"

# Generate digest
curl "http://localhost:9077/api/v1/digest?contact_id=test"
```

### Telegram — Test with BotFather

```bash
# 1. Message your bot directly in Telegram
# 2. Check bot received it via getUpdates:
curl "https://api.telegram.org/bot<YOUR_BOT_TOKEN>/getUpdates"

# 3. Test memory archival:
curl -X POST http://localhost:9077/api/v1/messages \
  -H 'Content-Type: application/json' \
  -d '{"sender":"telegram-test","contact_id":"test","channel":"telegram","raw_content":"Test message","category":"LOGISTICS"}'
```

### WhatsApp — Test with WhatsApp Business Test Number

```bash
# WhatsApp Business API provides test phone numbers
# Use the test number from your Meta Developer dashboard

# Test memory archival:
curl -X POST http://localhost:9077/api/v1/messages \
  -H 'Content-Type: application/json' \
  -d '{"sender":"whatsapp-test","contact_id":"test","channel":"whatsapp","raw_content":"Test message","category":"LOGISTICS"}'
```

### Verify Full Pipeline (All Platforms)

```bash
# 1. Archive a test message
curl -X POST http://localhost:9077/api/v1/messages \
  -H 'Content-Type: application/json' \
  -d '{"sender":"test","contact_id":"pipeline-test","raw_content":"You always miss the deadlines","category":"NOISE","escalation_score":4}'

# 2. Archive another
curl -X POST http://localhost:9077/api/v1/messages \
  -H 'Content-Type: application/json' \
  -d '{"sender":"test","contact_id":"pipeline-test","raw_content":"Pickup is at 5pm Thursday","category":"LOGISTICS","extracted_logistics":"pickup 5pm Thursday"}'

# 3. Check escalation
curl "http://localhost:9077/api/v1/escalation?contact_id=pipeline-test"
# Should return score with NOISE factored in

# 4. Get digest (logistics only)
curl "http://localhost:9077/api/v1/digest?contact_id=pipeline-test"
# Should return only the logistics message, not the noise

# 5. Verify DB integrity
grey-rock-memory --db ~/.grey-rock/memory.db verify-db
# Should return: ALL HASHES VALID

# 6. Test forensic archive
grey-rock-memory --db ~/.grey-rock/memory.db archive-messages -o /tmp/test-archive.json
grey-rock-memory verify-archive /tmp/test-archive.json
# Should return: VALID

# 7. Clean up test data
grey-rock-memory --db ~/.grey-rock/memory.db forget --namespace grey-rock --pattern "pipeline-test"
rm /tmp/test-archive.json
```

---

## Easiest Method: Train via Chat

With OpenClaw connected, simply message your assistant:

> "Remember: Contact A's phone number is +15551234567"
> "Remember: Standing call with external parties every Tuesday at 3pm"
> "Remember: My communication style is casual — I use contractions and shorthand"

OpenClaw stores these as long-term memories in the Grey Rock Memory system automatically. No JSON, no YAML, no files to edit.

## After Testing: Go Live

1. Remove all test data: `grey-rock-memory --db ~/.grey-rock/memory.db forget --pattern "test"`
2. Load real training data: `grey-rock-memory --db ~/.grey-rock/memory.db train your-data.json`
3. Configure real contacts in `grey-rock-config.json`
4. Enable channel (Signal/Telegram/WhatsApp) in your AI platform
5. Monitor: `curl http://localhost:9077/api/v1/stats` daily

---

*Grey Rock Memory and [Grey Rock Protocol](https://github.com/johngalt2035-dev/grey-rock-protocol) work together as a symbiotic system, built upon [OpenClaw](https://openclaw.ai).*
