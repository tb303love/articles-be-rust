// src/dtos/mod.rs

// 1. Deklaracija postojećih modula za artikle
pub mod article_dtos; // Ako su ti tamo izdvojeni ArticleCreateRequest i ArticleResponse
pub use article_dtos::*; // Izvlači ih na krovni nivo

// 2. DODATO: Deklaracija i re-export za auth module
pub mod auth_dtos;
pub use auth_dtos::{AuthResponse, LoginRequest};
