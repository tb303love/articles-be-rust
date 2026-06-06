// src/context.rs
use sqlx::MySqlPool;
use std::sync::Arc;
use crate::repositories::refresh_token_repository::SqlxRefreshTokenRepository;
// Uvoz repozitorijuma i servisa za artikle
use crate::repositories::SqlxArticleRepository;
use crate::services::ArticleService;

// DODATO: Uvoz novih auth komponenti
use crate::repositories::SqlxUserRepository;
use crate::services::AuthService;

#[derive(Clone)]
pub struct AppContext {
    pub article_service: Arc<ArticleService>,
    pub auth_service: Arc<AuthService>, // DODATO: Javni Singleton spreman za upotrebu
}

impl AppContext {
    pub fn new(pool: MySqlPool) -> Self {
        // 1. Sklapanje repozitorijuma unutar kontejnera
        let article_repo = Arc::new(SqlxArticleRepository::new(pool.clone()));
        let user_repo = Arc::new(SqlxUserRepository::new(pool.clone())); // DODATO
        let token_repo = Arc::new(SqlxRefreshTokenRepository::new(pool.clone()));

        // 2. Ubrizgavanje repozitorijuma u servise
        let article_service = Arc::new(ArticleService::new(article_repo, pool.clone()));
        let auth_service = Arc::new(AuthService::new(user_repo, token_repo,pool.clone())); // DODATO

        Self {
            article_service,
            auth_service, // DODATO
        }
    }
}
