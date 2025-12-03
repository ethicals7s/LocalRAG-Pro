#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;
use tokio::process::Command;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::time::Duration;
use walkdir::WalkDir;
use uuid::Uuid;
use anyhow::Result;
use std::fs;
use std::process::Stdio;

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
  id: String,
  role: String,
  content: String,
  timestamp: Option<String>,
}

#[tauri::command]
async fn plugin_init_app() -> Result<(), String> {
  // create data dir
  let data_dir = tauri::api::path::app_data_dir(&tauri::Config::default(), true)
    .unwrap_or_else(|| PathBuf::from(".localragnpro-data"));
  if !data_dir.exists() {
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
  }
  Ok(())
}

/// Index a folder: walk files, extract text, generate embeddings and upsert to Lance (node helper)
#[tauri::command]
async fn index_folder(folder: String) -> Result<(), String> {
  let folder_path = std::path::Path::new(&folder);
  if !folder_path.exists() {
    return Err("Folder not found".into());
  }

  // Collect text from supported files
  let mut docs: Vec<(String, String)> = Vec::new();
  for entry in WalkDir::new(&folder).into_iter().filter_map(|e| e.ok()) {
    if entry.file_type().is_file() {
      let path = entry.path();
      if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        match ext.to_lowercase().as_str() {
          "txt" | "md" | "rs" | "js" | "ts" | "py" | "java" => {
            if let Ok(s) = std::fs::read_to_string(path) {
              docs.push((path.display().to_string(), s))
            }
          }
          "pdf" => {
            match extract_text_from_pdf(path).await {
              Ok(text) => docs.push((path.display().to_string(), text)),
              Err(_) => {
                // fallback note
                docs.push((path.display().to_string(), "[PDF] (no text extracted; install pdftotext)".to_string()))
              }
            }
          }
          _ => {}
        }
      }
    }
  }

  // For each doc, call Ollama embed, then upsert into Lance via node helper
  for (path, content) in docs {
    tokio::time::sleep(Duration::from_millis(30)).await;
    if content.trim().is_empty() {
      continue;
    }
    let _ = generate_and_upsert_embedding(&path, &content).await;
  }

  // Spawn a file watcher for auto-reload (non-blocking)
  let _ = spawn_folder_watcher(folder).await;

  Ok(())
}

async fn extract_text_from_pdf(path: &std::path::Path) -> Result<String> {
  // Try to use pdftotext (poppler-utils) for best results
  let p = path.to_string_lossy().to_string();
  let output = Command::new("pdftotext")
    .arg("-layout")
    .arg(&p)
    .arg("-")
    .output()
    .await;

  match output {
    Ok(o) if o.status.success() => {
      let txt = String::from_utf8_lossy(&o.stdout).to_string();
      if !txt.trim().is_empty() {
        return Ok(txt);
      }
    }
    _ => {
      // If pdftotext is missing or fails, return an Err so caller can fallback or note the limitation.
      return Err(anyhow::anyhow!("pdftotext failed"));
    }
  }

  Err(anyhow::anyhow!("pdftotext produced empty output"))
}

async fn generate_and_upsert_embedding(path: &str, content: &str) -> Result<()> {
  // Use Ollama CLI to embed with nomic-embed-text
  let output = Command::new("ollama")
    .args(&["embed", "nomic-embed-text", "--text"])
    .arg(content)
    .output()
    .await?;

  if !output.status.success() {
    println!("Ollama embed failed for {}: {:?}", path, output);
    return Ok(());
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  // Expect JSON like: { "embedding": [ ... ] }
  let embedding_vec: Option<Vec<f32>> = match serde_json::from_str::<serde_json::Value>(&stdout) {
    Ok(v) => v.get("embedding").and_then(|e| e.as_array().map(|arr| {
      arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect::<Vec<f32>>()
    })),
    Err(_) => None,
  };

  // fallback: cannot parse embedding => skip numeric embedding; still upsert with text only
  let id = Uuid::new_v4().to_string();
  let upsert_payload = serde_json::json!({
    "id": id,
    "path": path,
    "text": content,
    "embedding": embedding_vec
  });

  // Call node helper: send JSON via stdin and read response
  let helper_path = "src-tauri/lance_helper/index.js"; // relative to repo root (dev). For packaged apps, ensure helper included in resources.
  let mut child = Command::new("node")
    .arg(helper_path)
    .arg("upsert")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

  if let Some(mut stdin) = child.stdin.take() {
    let payload_str = serde_json::to_string(&upsert_payload)?;
    stdin.write_all(payload_str.as_bytes()).await?;
    stdin.shutdown().await?;
  }

  // read stdout (not strictly necessary for upsert)
  let out = child.wait_with_output().await?;
  if !out.status.success() {
    println!("Lance helper upsert failed: {:?}", String::from_utf8_lossy(&out.stderr));
  }

  Ok(())
}

async fn spawn_folder_watcher(folder: String) -> Result<()> {
  // Spawn a file watcher using notify that triggers re-index on changes
  tokio::task::spawn_blocking(move || {
    use notify::{RecommendedWatcher, RecursiveMode, Watcher, EventKind};
    let folder_clone = folder.clone();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
      match res {
        Ok(event) => {
          if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)) {
            println!("Change detected: {:?}. You may want to re-index: {}", event.paths, folder_clone);
            // For now we don't auto-reindex to avoid runaway cycles.
          }
        }
        Err(e) => println!("watch error: {:?}", e),
      }
    }).expect("failed to create watcher");
    watcher.watch(std::path::Path::new(&folder_clone), notify::RecursiveMode::Recursive).expect("watch failed");
    // keep thread alive
    loop { std::thread::sleep(std::time::Duration::from_secs(3600)); }
  });
  Ok(())
}

#[tauri::command]
async fn chat_query(query: String, history: Vec<ChatMessage>) -> Result<serde_json::Value, String> {
  // 1) embed the query
  let embed_out = Command::new("ollama")
    .args(&["embed", "nomic-embed-text", "--text"])
    .arg(&query)
    .output()
    .await
    .map_err(|e| e.to_string())?;

  if !embed_out.status.success() {
    return Err("Failed to embed query".into());
  }
  let embed_stdout = String::from_utf8_lossy(&embed_out.stdout);
  let embedding_vec: Option<Vec<f32>> = match serde_json::from_str::<serde_json::Value>(&embed_stdout) {
    Ok(v) => v.get("embedding").and_then(|e| e.as_array().map(|arr| {
      arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect::<Vec<f32>>()
    })),
    Err(_) => None,
  };

  // 2) Query Lance (via node helper) for top-K contexts
  let mut top_contexts: Vec<serde_json::Value> = Vec::new();
  if let Some(emb) = embedding_vec {
    let q_payload = serde_json::json!({
      "embedding": emb,
      "topK": 3
    });

    let helper_path = "src-tauri/lance_helper/index.js";
    let mut child = Command::new("node")
      .arg(helper_path)
      .arg("query")
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .spawn()
      .map_err(|e| e.to_string())?;

    if let Some(mut stdin) = child.stdin.take() {
      let payload_str = serde_json::to_string(&q_payload).map_err(|e| e.to_string())?;
      stdin.write_all(payload_str.as_bytes()).await.map_err(|e| e.to_string())?;
      stdin.shutdown().await.map_err(|e| e.to_string())?;
    }

    let out = child.wait_with_output().await.map_err(|e| e.to_string())?;
    if out.status.success() {
      if let Ok(s) = String::from_utf8(out.stdout) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
          if let Some(items) = v.get("results").and_then(|r| r.as_array()) {
            for it in items {
              top_contexts.push(it.clone())
            }
          }
        }
      }
    } else {
      println!("Lance query failed: {}", String::from_utf8_lossy(&out.stderr));
    }
  }

  // 3) Build prompt using top contexts & history
  let mut prompt = String::from("You are LocalRAG Pro, a helpful assistant. Use the provided context to answer the question.\n\nCONTEXT:\n");
  for c in &top_contexts {
    if let Some(text) = c.get("text").and_then(|t| t.as_str()) {
      prompt.push_str("---\n");
      prompt.push_str(text);
      prompt.push_str("\n\n");
    }
  }
  prompt.push_str("\nConversation:\n");
  for m in history {
    prompt.push_str(&format!("{}: {}\n", m.role, m.content));
  }
  prompt.push_str("\nUser: ");
  prompt.push_str(&query);
  prompt.push_str("\nAssistant:");

  // 4) Call Ollama run with prompt
  let mut child = Command::new("ollama")
    .arg("run")
    .arg("llama3.2")
    .arg("--prompt")
    .arg(&prompt)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| e.to_string())?;

  let output = child.wait_with_output().await.map_err(|e| e.to_string())?;
  let answer = if output.status.success() {
    String::from_utf8_lossy(&output.stdout).to_string()
  } else {
    format!("(failed to generate answer): {}", String::from_utf8_lossy(&output.stderr))
  };

  Ok(serde_json::json!({ "answer": answer, "sources": top_contexts }))
}

#[tauri::command]
async fn save_chat(messages: Vec<ChatMessage>) -> Result<(), String> {
  let data_dir = tauri::api::path::app_data_dir(&tauri::Config::default(), true)
    .unwrap_or_else(|| PathBuf::from(".localragnpro-data"));
  let chats_dir = data_dir.join("chats");
  std::fs::create_dir_all(&chats_dir).map_err(|e| e.to_string())?;
  let id = Uuid::new_v4().to_string();
  let file = chats_dir.join(format!("{}.json", id));
  let content = serde_json::to_string_pretty(&messages).map_err(|e| e.to_string())?;
  std::fs::write(file, content).map_err(|e| e.to_string())?;
  Ok(())
}

#[tauri::command]
async fn export_chat(messages: Vec<ChatMessage>) -> Result<serde_json::Value, String> {
  let data_dir = tauri::api::path::app_data_dir(&tauri::Config::default(), true)
    .unwrap_or_else(|| PathBuf::from(".localragnpro-data"));
  let exports_dir = data_dir.join("exports");
  std::fs::create_dir_all(&exports_dir).map_err(|e| e.to_string())?;
  let id = Uuid::new_v4().to_string();

  // JSON export
  let json_file = exports_dir.join(format!("{}.json", id));
  let json_content = serde_json::to_string_pretty(&messages).map_err(|e| e.to_string())?;
  std::fs::write(&json_file, json_content).map_err(|e| e.to_string())?;

  // Markdown export
  let md = messages
    .iter()
    .map(|m| format!("**{}** ({})\n\n{}\n\n", m.role, m.timestamp.clone().unwrap_or_default(), m.content))
    .collect::<String>();
  let md_file = exports_dir.join(format!("{}.md", id));
  std::fs::write(&md_file, md).map_err(|e| e.to_string())?;

  Ok(serde_json::json!({ "path": md_file.display().to_string(), "json": json_file.display().to_string() }))
}

#[tauri::command]
async fn set_license(key: String) -> Result<bool, String> {
  // stub license check - store locally for now
  let data_dir = tauri::api::path::app_data_dir(&tauri::Config::default(), true)
    .unwrap_or_else(|| PathBuf::from(".localragnpro-data"));
  std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
  let file = data_dir.join("license.key");
  std::fs::write(file, key).map_err(|e| e.to_string())?;
  Ok(true)
}

#[tauri::command]
async fn get_license() -> Result<String, String> {
  let data_dir = tauri::api::path::app_data_dir(&tauri::Config::default(), true)
    .unwrap_or_else(|| PathBuf::from(".localragnpro-data"));
  let file = data_dir.join("license.key");
  match std::fs::read_to_string(file) {
    Ok(s) => Ok(s),
    Err(_) => Ok(String::new())
  }
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      plugin_init_app,
      index_folder,
      chat_query,
      save_chat,
      export_chat,
      set_license,
      get_license
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
