use std::path::Path;
use crate::services::scanner::DeepMatcher;

/// Example of how to use the DeepMatcher in a background task
pub async fn scan_folder_example(folder_path: &Path, db_metadata: &[ModMetadata]) {
    // 1. Offload to blocking thread (CPU Intensive)
    let folder = folder_path.to_owned();
    let targets = db_metadata.to_vec();

    let best_match = tokio::task::spawn_blocking(move || {
        let matcher = DeepMatcher::new(&targets);
        
        // 2. Run Pipeline
        // L1 -> L2 -> L3 -> L4 automatically handled by .score()
        matcher.score(&folder) 
    })
    .await
    .expect("Task panicked");

    // 3. Handle Result
    match best_match {
        Some((mod_id, score)) if score > 80.0 => {
            println!("Match Found: {} (Score: {})", mod_id, score);
        }
        _ => {
            println!("No Match or Low Confidence");
        }
    }
}
