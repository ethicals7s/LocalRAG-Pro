import React, { useState } from 'react'
import { invoke } from '@tauri-apps/api/tauri'

type Message = {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp?: string
}

export default function ChatPanel() {
  const [messages, setMessages] = useState<Message[]>([])
  const [input, setInput] = useState('')

  async function send() {
    if (!input) return
    const now = new Date().toISOString()
    const userMsg: Message = { id: String(Date.now()), role: 'user', content: input, timestamp: now }
    setMessages((m) => [...m, userMsg])
    setInput('')
    // query backend: invoke 'chat_query' with content and history
    const resp = await invoke('chat_query', { query: userMsg.content, history: messages })
    // resp expected {answer: string, sources: []}
    const answer = (resp as any)?.answer || 'No answer.'
    const assistant: Message = { id: String(Date.now() + 1), role: 'assistant', content: answer, timestamp: new Date().toISOString() }
    setMessages((m) => [...m, assistant])
  }

  async function saveChat() {
    await invoke('save_chat', { messages })
    alert('Saved')
  }

  async function exportChat() {
    const result = await invoke('export_chat', { messages })
    alert('Exported to: ' + (result as any).path)
  }

  return (
    <main className="flex-1 p-6 flex flex-col">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg">Chat with your docs</h2>
        <div>
          <button className="mr-2 rounded bg-gray-800 px-3 py-1 text-sm" onClick={saveChat}>
            Save
          </button>
          <button className="rounded bg-gray-800 px-3 py-1 text-sm" onClick={exportChat}>
            Export
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-auto mb-4 space-y-4">
        {messages.map((m) => (
          <div key={m.id} className={m.role === 'user' ? 'text-right' : ''}>
            <div className={`inline-block p-3 rounded-lg ${m.role === 'user' ? 'bg-blue-600' : 'bg-gray-800'}`}>{m.content}</div>
            <div className="text-xs text-gray-500">{m.timestamp}</div>
          </div>
        ))}
      </div>

      <div className="flex">
        <input
          className="flex-1 rounded-l bg-gray-900 border border-gray-800 p-3"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault()
              send()
            }
          }}
          placeholder="Ask about the indexed documents..."
        />
        <button className="rounded-r bg-blue-600 px-4" onClick={send}>
          Send
        </button>
      </div>
    </main>
  )
}
