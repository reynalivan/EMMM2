# Async Tasks & specialized Threads

## 1. The Golden Rule

**NEVER BLOCK THE MAIN THREAD.**
If a command takes >10ms, it must be `async` or run in a separate thread.

## 2. Tokio (I/O Bound)

Use `tokio` for File I/O, Database queries, and Network requests.

```rust
#[tauri::command]
pub async fn scan_folder(path: String) -> Result<Vec<String>, String> {
    // This runs on Tokio's worker threads
    let files = tokio::fs::read_dir(path).await
        .map_err(|e| e.to_string())?;
    // ...
    Ok(results)
}
```

## 3. Std Thread (CPU Bound)

For heavy computation (Hashing, Image Processing, Deep Matcher), `tokio` threads can still be blocked. Use `std::thread::spawn` or `rayon`.

```rust
#[tauri::command]
pub async fn heavy_compute(data: Vec<u8>) -> Result<String, String> {
    // Offload to a dedicated OS thread
    let result = std::thread::spawn(move || {
        // Heavy calculation here
        calculate_hash(&data)
    }).join().map_err(|_| "Thread panic".to_string())?;

    Ok(result)
}
```

## 4. Long Running Tasks (Progress)

For tasks taking seconds/minutes, emit events instead of waiting.

```rust
#[tauri::command]
pub async fn start_long_job(window: tauri::Window) -> Result<(), String> {
    tokio::spawn(async move {
        for i in 0..100 {
            window.emit("progress", i).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        window.emit("job-complete", "Done").unwrap();
    });
    Ok(())
}
```
