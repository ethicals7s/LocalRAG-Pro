```markdown
# LocalRAG Pro

![LocalRAG Pro - demo placeholder](./assets/placeholder.gif)

LocalRAG Pro — a 100% offline, single-binary desktop RAG app built with Tauri (Rust) + React. Drag-and-drop folders with PDFs, .txt, .md, and code files. Embeddings via Ollama (nomic-embed-text), vector search via LanceDB (local), and chat using local LLMs (default: llama3.2). Beautiful dark UI with Tailwind + shadcn/ui.

Made by the EchoArena guy. Monetization suggestion: $29 one-time on Gumroad.

Important: This app is designed to be fully offline. It calls only local programs (Ollama, pdftotext) and uses a local LanceDB store (or safe fallback JSON store) — your data never leaves the device.

Demo GIF placeholder
- File: assets/placeholder.gif
- Replace that file with your recorded demo GIF showing: drag-drop folder → indexing → chat → save/export.
- (Currently a placeholder file and a text note is included.)

Highlights
- Drag-and-drop folder indexing with auto-reload on file changes
- Local embeddings via Ollama (nomic-embed-text)
- Local vector DB (LanceDB via Node helper) with JSON fallback
- Chat using local model (llama3.2 via Ollama)
- Save chats and export to Markdown/JSON
- License check stub for Gumroad activation
- Dark UI built with Tailwind + shadcn/ui
- Single-binary packaging via Tauri

Quick installer & run (developer / local)
1. Prerequisites
   - Node.js 18+
   - npm or pnpm
   - Rust + cargo
   - Tauri prerequisites per platform: https://tauri.app/v1/guides/getting-started/prerequisites
   - Ollama (local): https://ollama.com
   - pdftotext (poppler-utils) recommended for PDF extraction: 
     - macOS: `brew install poppler`
     - Ubuntu/Debian: `sudo apt-get install poppler-utils`
     - Windows: install poppler and add `pdftotext.exe` to PATH

2. Install Ollama models (local)
   ollama pull nomic/embedding-model:nomic-embed-text
   ollama pull llama3.2

3. Install frontend deps & helper deps
   # in repo root (this contains the frontend)
   npm install

   # install Node Lance helper dependencies
   cd src-tauri/lance_helper
   npm install

4. Development run (dev mode)
   # in repo root
   npm run tauri:dev
   This starts the Vite dev server and launches the Tauri app. The Rust backend will call Ollama, pdftotext (if available), and the Node Lance helper for upserts/queries.

5. Production build (bundle)
   npm run build
   npm run tauri:build
   The Tauri builder produces native installers. Confirm that `src-tauri/lance_helper` is included in the resources if you bundle the helper with the binary (you can copy the helper into the built app resources using Tauri config or packaging scripts).

How the pieces communicate
- Frontend (React) uses Tauri commands to ask the Rust backend to:
  - index a folder
  - run chat queries
  - save/export chats
  - set license key
- Rust backend:
  - walks the folder and extracts text (pdftotext for PDF or text read)
  - calls Ollama CLI to generate embeddings
  - calls the Node lance_helper (via node process) to upsert embeddings and run similarity queries
  - constructs a prompt using top-K contexts and calls Ollama run with llama3.2 to generate answers
  - stores chats in platform app-data dir and handles export
- Node helper:
  - tries to use official LanceDB JS SDK (@lancedb/collections) to store vectors persistently and run similarity queries
  - if LanceDB JS isn't available, falls back to a robust JSON file vector store with cosine similarity (guarantees offline operation)

PDF extraction
- The Rust backend attempts to run `pdftotext -layout file.pdf -` to extract text (recommended).
- If `pdftotext` isn't present, a lightweight fallback message is recorded and the file is still indexed with a note to the user. For production, you can embed a pure-Rust PDF library to extract text directly, or bundle `pdftotext`.

License stub
- There's a license check command that stores a license key locally (currently a stub). Hook this to Gumroad license verification or any license server later. The app is offline-first; license activation should be implemented as issuance + symmetric activation token.

Where data is stored
- Platform specific app data dir (via Tauri API), inside which we store:
  - embeddings (managed by the Node helper / LanceDB path)
  - chats (JSON)
  - exports
  - license file (if set)

Security & privacy
- All processing is local. The app does not phone home.
- If you add cloud sync or analytics later, make it opt-in.

Monetization & Gumroad
- Price suggestion: $29 one-time
- Create a Gumroad product with a single-use license or a UUID you generate and present to users.
- Licensing: on purchase, provide a license key; the app accepts the key and stores it locally. The check is currently stubbed to allow integration later.

Gumroad listing suggestion
- Title: LocalRAG Pro — Offline RAG Desktop (Single Binary)
- Short description: Chat with your files locally. PDFs, MD, text, and code. Ollama embeddings + LanceDB search. Privacy-first. $29 one-time.
- Long description (for the listing): LocalRAG Pro lets you drop any folder of documents, PDFs, markdown, and code and chat with them — completely offline. Uses Ollama for embeddings and models, LanceDB for fast local vector search, and a beautiful dark desktop UI built with Tauri + React. No cloud required. Licensed per user. Made by the EchoArena guy.

Contributing & support
- Report issues / feature requests.
- If you want additional paid features (cloud sync, multi-device, team), I can implement paid upgrades.

Enjoy LocalRAG Pro — now with LanceDB helper and PDF extraction.
```
