# Observer AI 👁️
## It's not spying... if it's for you 👀
Local Open-source micro-agents that observe, log and react, all while keeping your data private and secure.


[Observer App Link](https://app.observer-ai.com/)

- [Support me and the project!](https://buymeacoffee.com/roy3838)

An open-source platform for running local AI agents that observe your screen while preserving privacy.


[![GitHub Pages](https://img.shields.io/badge/GitHub%20Pages-Deployed-success)](https://roy3838.github.io/observer-ai)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

# 🚀 Take a quick look:

https://github.com/user-attachments/assets/27b2d8e5-59c0-438a-999c-fc54b8c2cb95

# 🏗️ Building Your Own Agent

Creating your own Observer AI agent is simple, and consist of three things:

* SENSORS - input that your model will have
* MODELS - models run by ollama or by Ob-Server
* TOOLS - functions for your model to use

## Quick Start

1. Navigate to the Agent Dashboard and click "Create New Agent"
2. Fill in the "Configuration" tab with basic details (name, description, model, loop interval)
3. Give your model a system prompt and Sensors! The current Sensors that exist are:
   * **Screen OCR** ($SCREEN_OCR) Captures screen content as text via OCR
   * **Screenshot** ($SCREEN_64) Captures screen as an image for multimodal models
   * **Agent Memory** ($MEMORY@agent_id) Accesses agents' stored information
   * **Clipboard** ($CLIPBOARD) It pastes the clipboard contents 
   * **Microphone**\* ($MICROPHONE) Captures the microphone and adds a transcription
   * **Screen Audio**\* ($SCREEN_AUDIO) Captures the audio transcription of screen sharing a tab.
   * **All audio**\* ($ALL_AUDIO) Mixes the microphone and screen audio and provides a complete transcription of both (used for meetings).

\* Uses a whisper model with transformers.js (only supports whisper-tiny english for now)

4. Decide what tools do with your models `response` in the Code Tab:
  * `notify(title, options)` – Send notifications  
  * `getMemory(agentId)*` – Retrieve stored memory (defaults to current agent)  
  * `setMemory(agentId, content)*` – Replace stored memory  
  * `appendMemory(agentId, content)*` – Add to existing memory  
  * `startAgent(agentId)*` – Starts an agent  
  * `stopAgent(agentId)*` – Stops an agent
  * `time()` - Gets current time
  * `sendEmail(content, email)` - Sends an email
  * `sendSms(content, phone_number)` - Sends an SMS to a phone number, format as e.g. sendSms("hello",+181429367")
  * `sendWhatsapp(content, phone_number)` - Sends a whatsapp message, IMPORTANT: temporarily to counter anti spam, Observer is sending only static messages disregarding "content" variable.
  * `startClip()` - Starts a recording of any video media and saves it to the recording Tab.
  * `stopClip()` - Stops an active recording
  * `markClip(label)` - Adds a label to any active recording that will be displayed in the recording Tab.

## Code Tab

The "Code" tab now offers a notebook-style coding experience where you can choose between JavaScript or Python execution:

### JavaScript (Browser-based)

JavaScript agents run in the browser sandbox, making them ideal for passive monitoring and notifications:

```javascript
// Remove Think tags for deepseek model
const cleanedResponse = response.replace(/<think>[\s\S]*?<\/think>/g, '').trim();

// Preserve previous memory
const prevMemory = await getMemory();

// Get time
const time = time();

// Update memory with timestamp
appendMemory(`[${time}] ${cleanedResponse}`);
```

> **Note:** any function marked with `*` takes an `agentId` argument.  
> If you omit `agentId`, it defaults to the agent that’s running the code.

### Python (Jupyter Server)

Python agents run on a Jupyter server with system-level access, enabling them to interact directly with your computer:

```python
#python <-- don't remove this!
print("Hello World!", response, agentId)

# Example: Analyze screen content and take action
if "SHUTOFF" in response:
    # System level commands can be executed here
    import os
    # os.system("command")  # Be careful with system commands!
```

The Python environment receives:
* `response` - The model's output
* `agentId` - The current agent's ID

## Jupyter Server Configuration

To use Python agents:

1. Run a Jupyter server on your machine
2. Configure the connection in the Observer AI interface:
   * Host: The server address (e.g., 127.0.0.1)
   * Port: The server port (e.g., 8888)
   * Token: Your Jupyter server authentication token
3. Test the connection using the "Test Connection" button
4. Switch to the Python tab in the code editor to write Python-based agents

# 🚀 Getting Started with Local Inference

https://github.com/user-attachments/assets/1d19c572-359a-4eff-b6b6-f1e3efab9d86


There are a couple of ways to get Observer up and running with local inference. We recommend using Docker for the simplest setup.

### Option 1: Full Docker Setup and Webapp (Recommended & Easiest)

This method uses Docker Compose to run Observer-Ollama and a local Ollama instance together in containers. This process makes all processing happen 100% in your computer.

**Prerequisites:**
*   [Docker](https://docs.docker.com/get-docker/) installed.
*   [Docker Compose](https://docs.docker.com/compose/install/) installed (often included with Docker Desktop).

**Instructions:**

1.  **Clone this repository (or download the `docker-compose.yml` file):**
    ```bash
    git clone https://github.com/Roy3838/Observer.git
    cd Observer
    docker-compose up --build
    ```
    
3.  **Access Observer:**
    *   **WebApp:** Open your browser to `https://app.observer-ai.com`
    *   **Accept Local Certificates** Open up `https://localhost:3838` and your browser will show a warning about an "unsafe" or "untrusted" connection. This is because the proxy uses a self-signed SSL certificate for local HTTPS. You'll need to click "Advanced" and "Proceed to localhost (unsafe)" (or similar wording) to accept it. These certificates are signed by your computer! and this step is needed to make the browser happy and let it "see" the ollama server.

4.  **Pull Ollama Models:**
    Once the services are running, you can pull models into your Ollama instance using the terminal feature in the Observer UI, or by running:
    ```bash
    docker-compose exec ollama_service ollama pull llama3 # Or any other model
    ```
    OR by **Using the Web App:**
    *   Go to the Web UI (`https://app.observer-ai.com`).
    *   In the Models tab, click on add model. This will give you the shell to your connected ollama instance, download models using ollama run. 

**To Stop Observer (Docker Setup):**
```bash
docker-compose down
```
### Option 2: Full Docker Offline Setup 

This method is the same as the Full docker setup, but accessing `https://localhost:8080` for the webapp instead of `https://app.observer-ai.com` for serving. 

This works as a 100% offline alternative, but because of the offline "unsecure" environment (it is secure it just isn't https), Auth0 will complain; so the sendSms, sendWhatsapp and sendEmail tools won't work. 

I recommend going with Option 1 (it is 100% local) to have all of the Auth0 features. But i still wanted to give the option to self-host the webpage. 

## Deploy & Share

Save your agent, test it from the dashboard, and export the configuration to share with others!

## 🤝 Contributing

We welcome contributions from the community! Here's how you can help:

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

- [Website](https://observer-ai.com)
- [GitHub Repository](https://github.com/Roy3838/Observer)
- [twitter](https://x.com/AppObserverAI)

## 📧 Contact

- GitHub: [@Roy3838](https://github.com/Roy3838)
- Project Link: [https://observer-ai.com](https://observer-ai.com)

---

Built with ❤️  by Roy Medina for the Observer AI Community
Special thanks to the Ollama team for being an awesome backbone to this project!
