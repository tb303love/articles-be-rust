use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
pub struct ArticleComposition {
    // 1. Najveći tipovi (8 bajtova)
    pub id: Option<u64>,   // BIGINT UNSIGNED
    pub bundle_id: u64,    // "Dete" (Virtuelni artikal / paket)
    pub component_id: u64, // "Roditelj" (Fizički artikal koji drži zalihe)

    // 2. Najmanji tipovi (4 bajta)
    pub quantity: u32, // Količina komponente u paketu (ne može biti negativna)
}
