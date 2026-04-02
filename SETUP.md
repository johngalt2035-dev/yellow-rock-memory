# Getting Started with Yellow Rock

Set up your Executive Assistant utilizing the Yellow Rock communications protocol in 5 minutes. No technical background required.

---

## Step 1: Choose Your AI Assistant (2 minutes)

Yellow Rock works with any AI. Pick one and get an API key:

| Provider | Best For | Cost | Get API Key |
|----------|----------|------|-------------|
| **OpenAI (ChatGPT)** | Most popular, reliable | ~$0.01/message | [platform.openai.com/api-keys](https://platform.openai.com/api-keys) |
| **Anthropic (Claude)** | Excellent instruction following | ~$0.01/message | [console.anthropic.com](https://console.anthropic.com/) |
| **xAI (Grok)** | Fast, reasoning capable | ~$0.005/message | [console.x.ai](https://console.x.ai/) |
| **Google (Gemini)** | Good free tier | Free tier available | [aistudio.google.com](https://aistudio.google.com/) |
| **Ollama (Local)** | Free, runs on your computer | Free | [ollama.com](https://ollama.com/) |

Save your API key somewhere safe. You'll need it in Step 3.

## Step 2: Choose Your Messaging Platform

| Platform | Setup Difficulty | Best For |
|----------|-----------------|----------|
| **Signal** | Medium | Privacy-focused, encrypted |
| **Telegram** | Easy | Quick bot setup, groups |
| **WhatsApp** | Hard | Most widely used globally |
| **SMS** | Medium | Universal fallback, works with any phone |

See the platform-specific guides in `templates/channels/` for detailed setup instructions.

## Step 3: Configure Your Contacts (3 minutes)

Open the file `yellow-rock-config.json` in any text editor and fill in:

### Your Info
```json
"user": {
  "name": "Your First Name",
  "timezone": "America/New_York"
}
```

### Your Contacts

For each high-conflict individual, group, or business entity, add an entry:
```json
{
  "id": "opposing-party",
  "name": "Their Name",
  "phone": "+15551234567",
  "channel": "signal",
  "protocol": "yellow-rock"
}
```

For normal contacts (colleagues, professional contacts you communicate with normally):
```json
{
  "id": "colleague-a",
  "name": "Colleague Name",
  "phone": "+15559876543",
  "channel": "signal",
  "protocol": "normal"
}
```

### Communication Style

Choose a default style and per-contact overrides:

- **executive** -- formal, brief, factual. Suitable for high-conflict contacts.
- **personal** -- warm, natural, conversational. Suitable for friends and colleagues.

```json
"communication_style": {
  "default": "executive",
  "_options": ["personal", "executive"]
}
```

Per-contact override (in each contact entry):
```json
"communication_style": "personal"
```

### Your AI Choice
```json
"llm": {
  "provider": "openai",
  "model": "gpt-4o",
  "api_key_env": "OPENAI_API_KEY"
}
```

### Your Schedule
```json
"message_window": {
  "active_start": "07:00",
  "active_end": "22:00",
  "timezone": "America/New_York"
}
```

Messages are only sent during this window. Outside these hours, the system is silent.

## Step 4: Install Yellow Rock Memory (5-20 minutes)

Follow the installation guide for your platform: **[INSTALL.md](INSTALL.md)**
- [macOS](#macos) | [Windows](#windows) | [Ubuntu](#ubuntu--debian) | [Fedora](#fedora--rhel--centos)

Or let your AI do it: **[AI-INSTALL.md](AI-INSTALL.md)**

Once installed, start the memory system:
```bash
# Mac / Linux:
yellow-rock-memory --db ~/.yellow-rock/memory.db serve --port 9077

# Windows (PowerShell):
yellow-rock-memory --db "$env:USERPROFILE\.yellow-rock\memory.db" serve --port 9077
```

### Train with background knowledge (optional)

Create a file `background.json` with facts the system should know:
```json
[
  { "title": "Meeting schedule", "content": "Weekly sync Mon/Wed. Review meetings Tue/Thu." },
  { "title": "Conference call", "content": "Standing call at 3:15 PM with external parties." }
]
```

Then import:
```bash
yellow-rock-memory --db ~/.yellow-rock/memory.db train background.json
```

## Step 5: Test It

Send a test message and verify the system responds appropriately:

- **Grey rock contacts**: Should get brief, neutral, factual responses
- **Normal contacts**: Should get natural, friendly responses
- **During silent window**: No responses until the active window opens

## Frequently Asked Questions

**Q: Can the high-conflict counterparty tell it's AI?**
The system is designed to mirror your principal's communication style. Over time, populate the memory with preferences and patterns.

**Q: What if there's a real emergency?**
Level 5 emergencies bypass all protocols. The system responds immediately with appropriate urgency.

**Q: Is this legal?**
Consult a licensed attorney in your jurisdiction. Laws about AI-generated communications vary. See [LEGAL_DISCLAIMER.md](LEGAL_DISCLAIMER.md).

**Q: Can I use this for legal documentation?**
The forensic archive system provides SHA-256 hash verification, but consult your attorney about admissibility in your jurisdiction.

**Q: What AI provider should I choose?**
- **Budget**: Ollama (free, runs locally)
- **Best quality**: Claude or GPT-4o
- **Fastest**: xAI Grok
- **Free cloud**: Google Gemini free tier

**Q: Can I add more contacts later?**
Yes. Edit `yellow-rock-config.json` and add entries to the `contacts` array.
