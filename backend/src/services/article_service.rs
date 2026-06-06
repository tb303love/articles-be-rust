// src/services/article_service.rs (Deo 1)
use crate::dtos::{ArticleCreateRequest, ArticleResponse};
use crate::models::{Article, ArticleComposition, ArticleStock};
use crate::repositories::ArticleRepository;
use sqlx::{MySql, MySqlPool, Transaction};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ArticleService {
    article_repository: Arc<dyn ArticleRepository + Send + Sync>,
    private_pool: MySqlPool,
}

impl ArticleService {
    pub fn new(
        article_repository: Arc<dyn ArticleRepository + Send + Sync>,
        pool: MySqlPool,
    ) -> Self {
        Self {
            article_repository,
            private_pool: pool,
        }
    }

    // ==========================================
    // 1. CREATE ARTICLE (Čista biznis logika)
    // ==========================================
    pub async fn create_article(&self, dto: ArticleCreateRequest) -> Result<i64, sqlx::Error> {
        let mut tx: Transaction<'_, MySql> = self.private_pool.begin().await?;

        let article = Article {
            id: None,
            name: dto.name.clone(),
            active: dto.active,
            price: dto.price,
            admission_price_1: dto.admission_price_1,
            admission_price_2: dto.admission_price_2,
            category_id: dto.category_id,
            article_image_id: dto.article_image_id,
        };

        // Pozivamo repozitorijum i re-pozajmljujemo transakciju preko tx.as_mut()
        let article_id = self.article_repository.save(&article, tx.as_mut()).await?;

        // Upis bar-kodova
        self.article_repository.save_barcodes(article_id, &dto.barcodes, tx.as_mut()).await?;

        let is_bundle_request = dto.components.as_ref().map_or(false, |c| !c.is_empty());

        if is_bundle_request {
            if let Some(components) = &dto.components {
                self.article_repository.save_compositions(article_id, components, tx.as_mut()).await?;
            }
        } else {
            if let Some(stocks) = &dto.initial_stocks {
                self.article_repository.save_stocks(article_id, stocks, tx.as_mut()).await?;
            }
        }

        tx.commit().await?;
        Ok(article_id)
    }

    // ==========================================
    // 2. UPDATE ARTICLE (Čista biznis logika)
    // ==========================================
    pub async fn update_article(
        &self,
        article_id: i64,
        dto: ArticleCreateRequest,
    ) -> Result<(), sqlx::Error> {
        let mut tx: Transaction<'_, MySql> = self.private_pool.begin().await?;

        // Provera da li artikal uopšte postoji pre nego što krenemo sa brisanjem relacija
        let _article = self
            .article_repository
            .find_by_id(article_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let updated_article = Article {
            id: Some(article_id),
            name: dto.name.clone(),
            active: dto.active,
            price: dto.price,
            admission_price_1: dto.admission_price_1,
            admission_price_2: dto.admission_price_2,
            category_id: dto.category_id,
            article_image_id: dto.article_image_id,
        };

        // Orphan Removal starih bar-kodova
        self.article_repository.delete_barcodes(article_id, tx.as_mut()).await?;

        // Ažuriranje glavnog entiteta (sada se poziva samo jednom, popravljeno!)
        let article_id = self.article_repository.save(&updated_article, tx.as_mut()).await?;

        // Upis novih bar-kodova
        self.article_repository.save_barcodes(article_id, &dto.barcodes, tx.as_mut()).await?;

        let has_components_in_request = dto.components.as_ref().map_or(false, |c| !c.is_empty());
        let has_stocks_in_request = dto.initial_stocks.as_ref().map_or(false, |s| !s.is_empty());

        if has_components_in_request {
            self.article_repository.delete_stocks(article_id, tx.as_mut()).await?;
            self.article_repository.delete_compositions(article_id, tx.as_mut()).await?;

            if let Some(components) = &dto.components {
                self.article_repository.save_compositions(article_id, components, tx.as_mut()).await?;
            }
        } else if has_stocks_in_request {
            self.article_repository.delete_compositions(article_id, tx.as_mut()).await?;
            self.article_repository.delete_stocks(article_id, tx.as_mut()).await?;

            if let Some(stocks) = &dto.initial_stocks {
                self.article_repository.save_stocks(article_id, stocks, tx.as_mut()).await?;
            }
        } else {
            self.article_repository.delete_stocks(article_id, tx.as_mut()).await?;
            self.article_repository.delete_compositions(article_id, tx.as_mut()).await?;
        }

        tx.commit().await?;
        Ok(())
    }
    // src/services/article_service.rs (Deo 2)

        // ==========================================
        // 3. GET ALL ARTICLES (Bulk spajanje u memoriji)
        // ==========================================
        pub async fn get_all_articles(
            &self,
            query: Option<String>,
        ) -> Result<Vec<ArticleResponse>, sqlx::Error> {
            let articles = self.article_repository.search(query).await?;

            if articles.is_empty() {
                return Ok(Vec::new());
            }

            let article_ids: Vec<i64> = articles.iter().filter_map(|a| a.id).collect();

            // Masovno povlačenje relacija u jednoj turi iz baze podataka (N+1 rešen!)
            let barcode_rows = self.article_repository.find_barcodes_by_article_ids(&article_ids).await?;
            let stock_rows = self.article_repository.find_stocks_by_article_ids(&article_ids).await?;
            let comp_rows = self.article_repository.find_compositions_by_bundle_ids(&article_ids).await?;

            // Grupisanje bar-kodova po ID-ju artikla
            let mut barcodes_map: HashMap<i64, Vec<String>> = HashMap::new();
            for (article_id, barcode) in barcode_rows {
                barcodes_map.entry(article_id).or_default().push(barcode);
            }

            // Grupisanje zaliha
            let mut stocks_map: HashMap<i64, Vec<ArticleStock>> = HashMap::new();
            for s in stock_rows {
                stocks_map.entry(s.article_id).or_default().push(s);
            }

            // Grupisanje kompozicija paketa
            let mut comps_map: HashMap<i64, Vec<ArticleComposition>> = HashMap::new();
            for c in comp_rows {
                comps_map.entry(c.bundle_id).or_default().push(c);
            }

            let mut response = Vec::with_capacity(articles.len());

            for a in articles {
                let id = a.id.unwrap_or(0);

                // Izvlačimo nizove i prebacujemo vlasništvo bez alokacija (.clone())
                let current_barcodes = barcodes_map.remove(&id).unwrap_or_default();
                let current_stocks = stocks_map.remove(&id).unwrap_or_default();
                let current_comps = comps_map.remove(&id).unwrap_or_default();

                // Sklapanje DTO odgovora preko from_entity_with_barcodes metode
                let item_response = ArticleResponse::from_entity_with_barcodes(
                    a,
                    &current_stocks,
                    &current_comps,
                    current_barcodes,
                );

                response.push(item_response);
            }

            Ok(response)
        }

        // ==========================================
        // 4. CHECK NAME AVAILABILITY
        // ==========================================
        pub async fn check_name_availability(&self, name: &str) -> Result<bool, sqlx::Error> {
            let exists = self.article_repository.check_name_exists(name).await?;
            Ok(!exists)
        }

        // ==========================================
        // 5. CHECK NAME EXCLUDE SPECIFIC
        // ==========================================
        pub async fn find_article_by_article_name_exclude_specific(
            &self,
            name: &str,
            article_id: i64,
        ) -> Result<bool, sqlx::Error> {
            let exists = self.article_repository.check_name_exists_exclude(name, article_id).await?;
            Ok(!exists)
        }

        // ==========================================
        // 6. FIND BY BARCODE (Skeniranje na kasi)
        // ==========================================
        pub async fn find_by_barcode(&self, barcode: &str) -> Result<ArticleResponse, sqlx::Error> {
            let article_id = self.article_repository.find_id_by_barcode(barcode).await?;

            let article = self
                .article_repository
                .find_by_id(article_id)
                .await?
                .ok_or(sqlx::Error::RowNotFound)?;

            // Koristimo iste bulk metode prosleđujući niz sa jednim ID-jem radi uniformnosti
            let barcode_rows = self.article_repository.find_barcodes_by_article_ids(&[article_id]).await?;
            let stock_rows = self.article_repository.find_stocks_by_article_ids(&[article_id]).await?;
            let comp_rows = self.article_repository.find_compositions_by_bundle_ids(&[article_id]).await?;

            let barcodes: Vec<String> = barcode_rows.into_iter().map(|(_, bc)| bc).collect();

            Ok(ArticleResponse::from_entity_with_barcodes(
                article,
                &stock_rows,
                &comp_rows,
                barcodes,
            ))
        }

        // ==========================================
        // 7. GET AFFECTED BUNDLES
        // ==========================================
        pub async fn get_affected_bundles(&self, stock_id: i64) -> Result<Vec<String>, sqlx::Error> {
            let bundle_names = self.article_repository.get_affected_bundle_names(stock_id).await?;
            Ok(bundle_names)
        }

        // ==========================================
        // 8. DELETE BY ID
        // ==========================================
        pub async fn delete_by_id(&self, article_id: i64) -> Result<(), sqlx::Error> {
            let mut tx = self.private_pool.begin().await?;

            // Orphan removal zavisnih tabela unutar transakcije
            self.article_repository.delete_barcodes(article_id, tx.as_mut()).await?;
            self.article_repository.delete_stocks(article_id, tx.as_mut()).await?;
            self.article_repository.delete_compositions(article_id, tx.as_mut()).await?;

            // Brisanje samog artikla na kraju
            self.article_repository.delete(article_id, tx.as_mut()).await?;

            tx.commit().await?;
            Ok(())
        }
    } // Kraj impl bloka
