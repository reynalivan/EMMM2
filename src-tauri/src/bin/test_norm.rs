fn main() {
    let name = "ambercn";
    let norm = deunicode::deunicode(name);
    println!("name: {}, norm: {}", name, norm);
}
