import React, { useEffect, useState } from 'react'
import Sidebar from './components/Sidebar'
import ChatPanel from './components/Chat'
import { invoke } from '@tauri-apps/api/tauri'

export default function App() {
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null)
  const [indexed, setIndexed] = useState(false)
  const [license, setLicense] = useState<string | null>(null)

  useEffect(() => {
    // On app start, initialize storage
    invoke('plugin_init_app').catch(console.error)

    // load license if set
    invoke('get_license')
      .then((l: any) => {
        if (l && typeof l === 'string' && l.length > 0) setLicense(l)
      })
      .catch(() => {})
  }, [])

  async function handleDrop(path: string) {
    setSelectedFolder(path)
    setIndexed(false)
    try {
      // ask backend to add folder and index
      await invoke('index_folder', { folder: path })
      setIndexed(true)
    } catch (e) {
      console.error(e)
      alert('Indexing failed: ' + String(e))
    }
  }

  return (
    <div className="app-shell">
      <Sidebar onFolderDrop={handleDrop} selectedFolder={selectedFolder} indexed={indexed} license={license} />
      <ChatPanel />
    </div>
  )
}
