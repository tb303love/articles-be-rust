// src/context.rs
use crate::config::AppConfig;
use crate::repositories::{
   SqlxArticleImageRepository, SqlxArticleRepository, SqlxBarcodeRepository,
   SqlxCategoryRepository, SqlxCompositionRepository, SqlxRefreshTokenRepository,
   SqlxStockRepository, SqlxUserRepository,
};
use crate::services::{barcode_service::BarcodeService, ArticleService, AuthService};
use crate::ws::hub::WsHub;
use sqlx::MySqlPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppContext {
    pool: MySqlPool,
    article_service: Arc<ArticleService>,
    barcode_service: Arc<BarcodeService>,
    auth_service: Arc<AuthService>,
    ws_hub: Arc<WsHub>,
}

impl AppContext {
    pub fn new(pool: MySqlPool, config: Arc<AppConfig>) -> Self {
        // 1. Repozitorijumi
        let barcode_repo = Arc::new(SqlxBarcodeRepository::new(pool.clone()));
        let user_repo = Arc::new(SqlxUserRepository::new(pool.clone()));
        let token_repo = Arc::new(SqlxRefreshTokenRepository::new(pool.clone()));
        let article_repo = Arc::new(SqlxArticleRepository::new(pool.clone()));
        let composition_repo = Arc::new(SqlxCompositionRepository::new(pool.clone()));
        let stock_repo = Arc::new(SqlxStockRepository::new(pool.clone()));
        let article_image_repo = Arc::new(SqlxArticleImageRepository::new(pool.clone()));
        let category_repo = Arc::new(SqlxCategoryRepository::new(pool.clone()));

        let barcode_service = Arc::new(BarcodeService::new(barcode_repo.clone()));
        let auth_service = Arc::new(AuthService::new(user_repo, token_repo, config.clone()));
        let article_service = Arc::new(ArticleService::new(
            article_repo,
            article_image_repo,
            barcode_repo,
            composition_repo,
            stock_repo,
            category_repo,
        ));

        Self {
            pool,
            article_service,
            barcode_service,
            auth_service,
            ws_hub: WsHub::new(),
        }
    }

    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    pub fn barcode_service(&self) -> Arc<BarcodeService> {
        self.barcode_service.clone()
    }

    pub fn auth_service(&self) -> Arc<AuthService> {
        self.auth_service.clone()
    }

    pub fn article_service(&self) -> Arc<ArticleService> {
        self.article_service.clone()
    }

    pub fn ws_hub(&self) -> Arc<WsHub> {
        self.ws_hub.clone()
    }
}
