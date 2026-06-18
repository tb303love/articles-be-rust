// src/services/article_service.rs (DEO 1)

use crate::dto::{ArticleCreateRequest, ArticleResponse};
use crate::models::{Article, ArticleComposition, ArticleStock};
use crate::repositories::{
    SqlxArticleRepository, SqlxBarcodeRepository, SqlxCompositionRepository, SqlxStockRepository,
};
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ArticleService {
    article_repo: Arc<SqlxArticleRepository>,
    barcode_repo: Arc<SqlxBarcodeRepository>,
    composition_repo: Arc<SqlxCompositionRepository>,
    stock_repo: Arc<SqlxStockRepository>,
}

impl ArticleService {
    pub fn new(
        article_repo: Arc<SqlxArticleRepository>,
        barcode_repo: Arc<SqlxBarcodeRepository>,
        composition_repo: Arc<SqlxCompositionRepository>,
        stock_repo: Arc<SqlxStockRepository>,
    ) -> Self {
        Self {
            article_repo,
            barcode_repo,
            composition_repo,
            stock_repo,
        }
    }

    pub async fn create_article(
        &self,
        pool: &MySqlPool,
        dto: ArticleCreateRequest,
    ) -> Result<i64, sqlx::Error> {
        // 1. Validacija poslovnog pravila (fail-fast)
        if let (Some(comps), Some(stocks)) = (&dto.components, &dto.initial_stocks) {
            if !comps.is_empty() && !stocks.is_empty() {
                return Err(sqlx::Error::Protocol(
                    "Artikal ne može istovremeno imati komponente (sastavnicu) i početne zalihe!"
                        .into(),
                ));
            }
        }

        // 2. Otvaranje transakcije i bezbedno izvlačenje konekcije
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        // 3. Rešavanje kategorije preko njene dve tabele
        let category_id = self.handle_category(&dto.category_name, conn).await?;

        let article = Article {
            id: None,
            name: dto.name.clone(),
            active: dto.active,
            price: dto.price,
            admission_price_1: dto.admission_price_1,
            admission_price_2: dto.admission_price_2,
            category_id,
            article_image_id: dto.article_image_id,
        };

        // 4. Upis glavnog artikla
        let article_id = self.article_repo.save(&article, conn).await?;

        // 5. Masovni upis bar-kodova
        self.barcode_repo
            .save_barcodes(article_id, &dto.barcodes, conn)
            .await?;

        // 6. Ekskluzivna logika na osnovu prisustva polja
        let has_components = dto.components.as_ref().map_or(false, |c| !c.is_empty());
        let has_stocks = dto.initial_stocks.as_ref().map_or(false, |s| !s.is_empty());

        if has_components {
            if let Some(components) = &dto.components {
                self.composition_repo
                    .save_compositions(article_id, components, conn)
                    .await?;
            }
        } else if has_stocks {
            if let Some(stocks) = &dto.initial_stocks {
                self.stock_repo
                    .save_stocks(article_id, stocks, conn)
                    .await?;
            }
        }

        tx.commit().await?;

        Ok(article_id)
    }

    async fn handle_category(
        &self,
        category_name: &Option<String>,
        conn: &mut sqlx::MySqlConnection,
    ) -> Result<Option<i64>, sqlx::Error> {
        if let Some(cat_name) = category_name {
            let id = self
                .article_repo
                .get_or_create_category(cat_name, conn)
                .await?;
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }
    // src/services/article_service.rs (DEO 2)

    pub async fn update_article(
        &self,
        pool: &MySqlPool,
        article_id: i64,
        dto: ArticleCreateRequest,
    ) -> Result<(), sqlx::Error> {
        // 1. Validacija poslovnog pravila
        if let (Some(comps), Some(stocks)) = (&dto.components, &dto.initial_stocks) {
            if !comps.is_empty() && !stocks.is_empty() {
                return Err(sqlx::Error::Protocol(
                    "Artikal ne može istovremeno imati komponente i zalihe!".into(),
                ));
            }
        }

        // 2. Otvaranje transakcije i izvlačenje konekcije
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        // 3. Provera da li artikal uopšte postoji
        let article_opt = self.article_repo.find_by_id(article_id).await?;

        if article_opt.is_none() {
            return Err(sqlx::Error::RowNotFound);
        }

        let category_id = self.handle_category(&dto.category_name, conn).await?;

        let article = Article {
            id: Some(article_id),
            name: dto.name.clone(),
            active: dto.active,
            price: dto.price.clone(),
            admission_price_1: dto.admission_price_1.clone(),
            admission_price_2: dto.admission_price_2.clone(),
            category_id,
            article_image_id: dto.article_image_id,
        };

        self.article_repo.save(&article, conn).await?;

        // 6. Sinhronizacija barkodova (Brišemo stare, upisujemo nove)
        self.barcode_repo.delete_barcodes(article_id, conn).await?;
        self.barcode_repo
            .save_barcodes(article_id, &dto.barcodes, conn)
            .await?;

        // 7. Ekskluzivna logika na osnovu prisustva polja
        let has_components = dto.components.as_ref().map_or(false, |c| !c.is_empty());
        let has_stocks = dto.initial_stocks.as_ref().map_or(false, |s| !s.is_empty());

        if has_components {
            self.stock_repo.delete_stocks(article_id, conn).await?;
            self.composition_repo
                .delete_compositions(article_id, conn)
                .await?;

            if let Some(components) = &dto.components {
                self.composition_repo
                    .save_compositions(article_id, components, conn)
                    .await?;
            }
        } else if has_stocks {
            self.composition_repo
                .delete_compositions(article_id, conn)
                .await?;
            self.stock_repo.delete_stocks(article_id, conn).await?;

            if let Some(stocks) = &dto.initial_stocks {
                self.stock_repo
                    .save_stocks(article_id, stocks, conn)
                    .await?;
            }
        } else {
            // Korisnik je namerno ispraznio i jedno i drugo polje
            self.stock_repo.delete_stocks(article_id, conn).await?;
            self.composition_repo
                .delete_compositions(article_id, conn)
                .await?;
        }

        // 8. Commit transakcije
        tx.commit().await?;
        Ok(())
    }
    // src/services/article_service.rs (DEO 3)

    pub async fn get_all_articles(
        &self,
        query: Option<String>,
    ) -> Result<Vec<ArticleResponse>, sqlx::Error> {
        // 1. Povlačimo glavne artikle na osnovu pretrage
        let articles = self.article_repo.search(query).await?;

        // Ako je baza prazna, odmah vraćamo prazan niz i prekidamo izvršavanje
        if articles.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Sakupljamo sve ID-jeve artikala da bismo masovno povukli njihove relacije
        let article_ids: Vec<i64> = articles.iter().filter_map(|a| a.id).collect();

        // 3. Masovno povlačenje relacija u jednoj turi iz baze podataka (N+1 rešen!)
        let barcode_rows = self.barcode_repo.find_by_article_ids(&article_ids).await?;
        let stock_rows = self
            .stock_repo
            .find_stocks_by_article_ids(&article_ids)
            .await?;
        let comp_rows = self
            .composition_repo
            .find_compositions_by_bundle_ids(&article_ids)
            .await?;

        // 4. Grupisanje bar-kodova po ID-ju artikla u memoriji
        let mut barcodes_map: HashMap<i64, Vec<String>> = HashMap::new();
        for (article_id, barcode) in barcode_rows {
            barcodes_map.entry(article_id).or_default().push(barcode);
        }

        // 5. Grupisanje zaliha po ID-ju artikla u memoriji
        let mut stocks_map: HashMap<i64, Vec<ArticleStock>> = HashMap::new();
        for stock in stock_rows {
            stocks_map.entry(stock.article_id).or_default().push(stock);
        }

        // 6. Grupisanje komponenti baze po ID-ju artikla (bundle_id) u memoriji
        let mut comps_map: HashMap<i64, Vec<ArticleComposition>> = HashMap::new();
        for comp in comp_rows {
            comps_map.entry(comp.bundle_id).or_default().push(comp);
        }

        // 7. Sklapanje konačnih odgovora preko DTO transformacije (O(1) kompleksnost pretrage)
        let mut responses = Vec::with_capacity(articles.len());
        for article in articles {
            let id = article.id.unwrap_or_default();

            // Izvlačimo sakupljene sirove reference iz mapa za trenutni artikal
            let art_barcodes = barcodes_map.get(&id).cloned().unwrap_or_default();
            let art_stocks = stocks_map.get(&id).cloned().unwrap_or_default();
            let art_comps = comps_map.get(&id).cloned().unwrap_or_default();

            // Pozivamo tvoju prerađenu metodu iz dto.rs koja sama mapira zalihe i komponente
            let response = ArticleResponse::from_entity_with_barcodes(
                article,
                &art_stocks,
                &art_comps,
                art_barcodes,
            );

            responses.push(response);
        }

        Ok(responses)
    }

    pub async fn check_name_availability(&self, name: &str) -> Result<bool, sqlx::Error> {
        let exists = self.article_repo.check_name_exists(name).await?;
        Ok(!exists)
    }

    pub async fn find_article_by_article_name_exclude_specific(
        &self,
        name: &str,
        id: i64,
    ) -> Result<bool, sqlx::Error> {
        let exists = self
            .article_repo
            .check_name_exists_exclude(name, id)
            .await?;
        Ok(!exists)
    }

    pub async fn get_affected_bundles(&self, stock_id: i64) -> Result<Vec<String>, sqlx::Error> {
        let bundle_names = self.stock_repo.find_affected_bundles(stock_id).await?;
        Ok(bundle_names)
    }

    pub async fn find_by_barcode(&self, code: &str) -> Result<ArticleResponse, sqlx::Error> {
        let article_id = self.barcode_repo.find_article_id_by_barcode(code).await?;

        let article = self
            .article_repo
            .find_by_id(article_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let barcodes = self.barcode_repo.find_by_article_ids(&[article_id]).await?;
        let stocks = self
            .stock_repo
            .find_stocks_by_article_ids(&[article_id])
            .await?;
        let compositions = self
            .composition_repo
            .find_compositions_by_bundle_ids(&[article_id])
            .await?;

        let art_barcodes: Vec<String> = barcodes.into_iter().map(|(_, b)| b).collect();

        let response = ArticleResponse::from_entity_with_barcodes(
            article,
            &stocks,
            &compositions,
            art_barcodes,
        );

        Ok(response)
    }

    pub async fn delete_article(
        &self,
        pool: &MySqlPool,
        article_id: i64,
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        self.barcode_repo.delete_barcodes(article_id, conn).await?;
        self.stock_repo.delete_stocks(article_id, conn).await?;
        self.composition_repo
            .delete_compositions(article_id, conn)
            .await?;
        self.article_repo.delete(article_id, conn).await?;

        tx.commit().await?;
        Ok(())
    }
}
