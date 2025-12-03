import React, { useCallback, useRef } from 'react'

export default function Sidebar({ onFolderDrop, selectedFolder, indexed, license }: any) {
  const inputRef = useRef<HTMLInputElement | null>(null)

  const onDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
  }, [])

  const onDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    const path = (e.dataTransfer as any).files?.[0]?.path
    if (path) onFolderDrop(path)
  }, [onFolderDrop])

  return (
    <aside className="w-80 p-4 border-r border-gray-800">
      <div className="mb-4">
        <h1 className="text-2xl font-semibold">LocalRAG Pro</h1>
        <p className="text-sm text-gray-400">Drop a folder to start indexing.</p>
      </div>

      <div
        className="h-48 rounded-lg border-2 border-dashed border-gray-800 flex items-center justify-center cursor-pointer"
        onDragOver={onDragOver}
        onDrop={onDrop}
        onClick={() => inputRef.current?.click()}
      >
        <input ref={inputRef} type="file" webkitdirectory="true" className="hidden" />
        <div className="text-center">
          <div className="text-gray-400">Drop folder here</div>
          {selectedFolder && <div className="text-xs mt-2 text-gray-300">{selectedFolder}</div>}
          <div className="mt-2 text-sm">
            {indexed ? <span className="text-green-400">Indexed ✓</span> : <span className="text-yellow-400">Not indexed</span>}
          </div>
        </div>
      </div>

      <div className="mt-6">
        <h2 className="text-sm text-gray-300 mb-2">Saved chats</h2>
        <div className="text-xs text-gray-500">No chats yet</div>
      </div>

      <div className="mt-6 text-xs text-gray-500">
        <strong>Local-only</strong> — models run on your machine (Ollama). See README for setup.
      </div>

      <div className="mt-6 text-xs text-gray-500">
        <strong>License</strong>
        <div>{license ? <span className="text-green-400">Activated</span> : <span className="text-red-400">Not activated</span>}</div>
      </div>
    </aside>
  )
}
