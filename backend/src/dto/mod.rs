// src/dto/mod.rs

// 1. Deklaracija postojećih modula za artikle
mod article_dto; // Ako su ti tamo izdvojeni ArticleCreateRequest i ArticleResponse
pub use article_dto::*; // Izvlači ih na krovni nivo

// 2. DODATO: Deklaracija i re-export za auth module
pub mod auth_dto;
pub mod barcode_dto;
pub mod order_dto;

pub use auth_dto::{AuthResponse, LoginRequest};
