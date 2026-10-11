#[cfg(windows)]
fn main() -> std::io::Result<()> {
    use std::io::{BufRead, Write};
    use std::os::windows::fs::OpenOptionsExt;
    let directory = std::path::PathBuf::from(std::env::args_os().nth(1).expect("owned directory"));
    let directory_handle = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .custom_flags(0x02000000)
        .open(&directory)?;
    let file_handle = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(directory.join("mod.ini"))?;
    println!("EMMM_OWNED_LOCK_READY");
    std::io::stdout().flush()?;
    let mut release = String::new();
    std::io::stdin().lock().read_line(&mut release)?;
    drop(file_handle);
    drop(directory_handle);
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    panic!("The owned handle fixture requires Windows");
}
