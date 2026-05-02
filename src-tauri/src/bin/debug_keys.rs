use emmm_lib::services::path_key::folder_path_key;

fn main() {
    let p1 = "E:\\Dev\\EMMMNEW\\src-tauri\\Mods\\Aether";
    let p2 = "E:\\Dev\\EMMMNEW\\src-tauri\\Mods\\Lumine";
    let k1 = folder_path_key(p1, None);
    let k2 = folder_path_key(p2, None);
    println!("P1: {} -> KEY: {}", p1, k1);
    println!("P2: {} -> KEY: {}", p2, k2);

    let p3 = "E:\\Dev\\EMMMNEW\\src-tauri\\Mods\\DISABLED Kazuha";
    let k3 = folder_path_key(p3, None);
    println!("P3: {} -> KEY: {}", p3, k3);
}
