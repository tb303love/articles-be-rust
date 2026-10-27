// Ponovo kompajliraj kada se promeni `migrations/` - `sqlx::migrate!` ugrađuje SQL fajlove u binarni fajl
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
