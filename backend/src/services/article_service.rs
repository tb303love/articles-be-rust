use crate::dtos::{ArticleCreateRequest, ArticleResponse};
use crate::models::{Article, ArticleComposition, ArticleStock};
use crate::repositories::{
    ArticleRepository, BarcodeRepository, CompositionRepository, StockRepository,
};
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ArticleService {
    article_repo: Arc<dyn ArticleRepository>,
    barcode_repo: Arc<dyn BarcodeRepository>,
    composition_repo: Arc<dyn CompositionRepository>,
    stock_repo: Arc<dyn StockRepository>,
    pool: MySqlPool,
}

impl ArticleService {
    pub fn new(
        article_repo: Arc<dyn ArticleRepository>,
        barcode_repo: Arc<dyn BarcodeRepository>,
        composition_repo: Arc<dyn CompositionRepository>,
        stock_repo: Arc<dyn StockRepository>,
        pool: MySqlPool,
    ) -> Self {
        Self {
            article_repo,
            barcode_repo,
            composition_repo,
            stock_repo,
            pool,
        }
    }

    pub async fn create_article(&self, dto: ArticleCreateRequest) -> Result<i64, sqlx::Error> {
        // 1. Validacija poslovnog pravila (fail-fast)
        if let (Some(comps), Some(stocks)) = (&dto.components, &dto.initial_stocks) {
            if !comps.is_empty() && !stocks.is_empty() {
                return Err(sqlx::Error::Protocol(
                    "Artikal ne može istovremeno imati komponente (sastavnicu) i početne zalihe!"
                        .into(),
                ));
            }
        }

        // 2. Otvaranje transakcije
        let mut tx = self.pool.begin().await?;

        // 3. Rešavanje kategorije preko njene dve tabele
        // Samo jedna linija menja ceo stari blok koda:
        let category_id = self
            .handle_category(&dto.category_name, tx.as_mut())
            .await?;

        let article = Article {
            id: None,
            name: dto.name.clone(),
            active: dto.active,
            price: dto.price,
            admission_price_1: dto.admission_price_1,
            admission_price_2: dto.admission_price_2,
            category_id, // <- Prosleđujemo id izvučen iz tabele 'category'
            article_image_id: dto.article_image_id,
        };

        // 4. Upis glavnog artikla
        let article_id = self.article_repo.save(&article, tx.as_mut()).await?;

        // 5. Masovni upis bar-kodova (QueryBuilder verzija bez zvezdica)
        self.barcode_repo
            .save_barcodes(article_id, &dto.barcodes, tx.as_mut())
            .await?;

        // 6. Ekskluzivna logika na osnovu prisustva polja
        let has_components = dto.components.as_ref().map_or(false, |c| !c.is_empty());
        let has_stocks = dto.initial_stocks.as_ref().map_or(false, |s| !s.is_empty());

        if has_components {
            if let Some(components) = &dto.components {
                self.composition_repo
                    .save_compositions(article_id, components, tx.as_mut())
                    .await?;
            }
        } else if has_stocks {
            if let Some(stocks) = &dto.initial_stocks {
                self.stock_repo
                    .save_stocks(article_id, stocks, tx.as_mut())
                    .await?;
            }
        }

        tx.commit().await?;

        Ok(article_id)
    }

    async fn handle_category(
        &self,
        category_name: &Option<String>,
        tx: &mut sqlx::MySqlConnection,
    ) -> Result<Option<i64>, sqlx::Error> {
        if let Some(cat_name) = category_name {
            let id = self
                .article_repo
                .get_or_create_category(cat_name, tx)
                .await?;
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }

    pub async fn update_article(
        &self,
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

        // 2. Otvaranje transakcije
        let mut tx = self.pool.begin().await?;

        // 3. Provera da li artikal uopšte postoji
        let article_opt = self.article_repo.find_by_id(article_id).await?;

        if article_opt.is_none() {
            return Err(sqlx::Error::RowNotFound);
        }

        let category_id = self
            .handle_category(&dto.category_name, tx.as_mut())
            .await?;

        let article = Article {
            id: Some(article_id), // <- Ključna razlika u odnosu na create (tamo je bilo None)
            name: dto.name.clone(),
            active: dto.active,
            price: dto.price.clone(),
            admission_price_1: dto.admission_price_1.clone(),
            admission_price_2: dto.admission_price_2.clone(),
            category_id,
            article_image_id: dto.article_image_id,
        };

        self.article_repo.save(&article, tx.as_mut()).await?;

        // 6. Sinhronizacija barkodova (Brišemo stare, upisujemo nove)
        self.barcode_repo
            .delete_barcodes(article_id, tx.as_mut())
            .await?;
        self.barcode_repo
            .save_barcodes(article_id, &dto.barcodes, tx.as_mut())
            .await?;

        // 7. Ekskluzivna logika na osnovu prisustva polja (Preslikano iz Spring-a)
        let has_components = dto.components.as_ref().map_or(false, |c| !c.is_empty());
        let has_stocks = dto.initial_stocks.as_ref().map_or(false, |s| !s.is_empty());

        if has_components {
            self.stock_repo
                .delete_stocks(article_id, tx.as_mut())
                .await?;
            self.composition_repo
                .delete_compositions(article_id, tx.as_mut())
                .await?;

            if let Some(components) = &dto.components {
                self.composition_repo
                    .save_compositions(article_id, components, tx.as_mut())
                    .await?;
            }
        } else if has_stocks {
            self.composition_repo
                .delete_compositions(article_id, tx.as_mut())
                .await?;
            self.stock_repo
                .delete_stocks(article_id, tx.as_mut())
                .await?;

            if let Some(stocks) = &dto.initial_stocks {
                self.stock_repo
                    .save_stocks(article_id, stocks, tx.as_mut())
                    .await?;
            }
        } else {
            // Korisnik je namerno ispraznio i jedno i drugo polje
            self.stock_repo
                .delete_stocks(article_id, tx.as_mut())
                .await?;
            self.composition_repo
                .delete_compositions(article_id, tx.as_mut())
                .await?;
        }

        // 8. Commit transakcije
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_all_articles(
        &self,
        query: Option<String>,
    ) -> Result<Vec<ArticleResponse>, sqlx::Error> {
        let articles = self.article_repo.search(query).await?;

        if articles.is_empty() {
            return Ok(Vec::new());
        }

        let article_ids: Vec<i64> = articles.iter().filter_map(|a| a.id).collect();

        // Masovno povlačenje relacija u jednoj turi iz baze podataka (N+1 rešen!)
        let barcode_rows = self
            .barcode_repo
            .find_barcodes_by_article_ids(&article_ids)
            .await?;
        let stock_rows = self
            .stock_repo
            .find_stocks_by_article_ids(&article_ids)
            .await?;
        let comp_rows = self
            .composition_repo
            .find_compositions_by_bundle_ids(&article_ids)
            .await?;

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

    pub async fn check_name_availability(&self, name: &str) -> Result<bool, sqlx::Error> {
        let exists = self.article_repo.check_name_exists(name).await?;
        Ok(!exists)
    }

    pub async fn find_article_by_article_name_exclude_specific(
        &self,
        name: &str,
        article_id: i64,
    ) -> Result<bool, sqlx::Error> {
        let exists = self
            .article_repo
            .check_name_exists_exclude(name, article_id)
            .await?;
        Ok(!exists)
    }

    pub async fn find_by_barcode(&self, barcode: &str) -> Result<ArticleResponse, sqlx::Error> {
        let article_id = self.barcode_repo.find_id_by_barcode(barcode).await?;

        let article = self
            .article_repo
            .find_by_id(article_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // Koristimo iste bulk metode prosleđujući niz sa jednim ID-jem radi uniformnosti
        let barcode_rows = self
            .barcode_repo
            .find_barcodes_by_article_ids(&[article_id])
            .await?;
        let stock_rows = self
            .stock_repo
            .find_stocks_by_article_ids(&[article_id])
            .await?;
        let comp_rows = self
            .composition_repo
            .find_compositions_by_bundle_ids(&[article_id])
            .await?;

        let barcodes: Vec<String> = barcode_rows.into_iter().map(|(_, bc)| bc).collect();

        Ok(ArticleResponse::from_entity_with_barcodes(
            article,
            &stock_rows,
            &comp_rows,
            barcodes,
        ))
    }

    pub async fn get_affected_bundles(&self, stock_id: i64) -> Result<Vec<String>, sqlx::Error> {
        let bundle_names = self
            .composition_repo
            .get_affected_bundle_names(stock_id)
            .await?;
        Ok(bundle_names)
    }

    pub async fn delete_by_id(&self, article_id: i64) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Orphan removal zavisnih tabela unutar transakcije
        self.barcode_repo
            .delete_barcodes(article_id, tx.as_mut())
            .await?;
        self.stock_repo
            .delete_stocks(article_id, tx.as_mut())
            .await?;
        self.composition_repo
            .delete_compositions(article_id, tx.as_mut())
            .await?;

        // Brisanje samog artikla na kraju
        self.article_repo.delete(article_id, tx.as_mut()).await?;

        tx.commit().await?;
        Ok(())
    }
}
