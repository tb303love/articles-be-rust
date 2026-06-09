// src/context.rs
use crate::config::AppConfigProvider;
use crate::services::ArticleService;
use sqlx::MySqlPool;
use std::sync::Arc;

// DODATO: Uvoz novih auth komponenti
use crate::repositories::{SqlxArticleRepository, SqlxBarcodeRepository, SqlxCompositionRepository, SqlxRefreshTokenRepository, SqlxStockRepository, SqlxUserRepository};
use crate::services::AuthService;

#[derive(Clone)]
pub struct AppContext {
    pub article_service: Arc<ArticleService>,
    pub auth_service: Arc<AuthService>, // DODATO: Javni Singleton spreman za upotrebu
}

impl AppContext {
    pub fn new(pool: MySqlPool, config: Arc<dyn AppConfigProvider>) -> Self {
        // 1. Kreiramo konkretne implementacije
        let article_repo = Arc::new(SqlxArticleRepository::new(pool.clone()));
        let barcode_repo = Arc::new(SqlxBarcodeRepository::new(pool.clone()));
        let composition_repo = Arc::new(SqlxCompositionRepository::new(pool.clone()));
        let stock_repo = Arc::new(SqlxStockRepository::new(pool.clone()));

        let user_repo = Arc::new(SqlxUserRepository::new(pool.clone()));
        let token_repo = Arc::new(SqlxRefreshTokenRepository::new(pool.clone()));

        // 2. Ubrizgavamo ih u servis (Rust ih sam prepoznaje kao Arc<dyn Trait>)
        let article_service = Arc::new(ArticleService::new(
            article_repo,
            barcode_repo,
            composition_repo,
            stock_repo,
            pool.clone(),
        ));

        let auth_service = Arc::new(AuthService::new(
            user_repo,
            token_repo,
            pool.clone(),
            config.clone(),
        ));

        Self {
            article_service,
            auth_service,
        }
    }
}
