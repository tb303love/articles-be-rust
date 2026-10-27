use crate::dto::{
   ArticleCategory, ArticleCreateRequest, ArticleResponse, ArticleStockInputDto,
   ArticleUpdateRequest, CategoryNameResponse, ComponentInputDto,
};
use crate::models::{Article, ArticleComposition, ArticleStock, ImageUploadData};
use crate::repositories::{
   SqlxArticleImageRepository, SqlxArticleRepository, SqlxBarcodeRepository,
   SqlxCategoryRepository, SqlxCompositionRepository, SqlxStockRepository,
};
use sqlx::{Error, MySqlConnection, MySqlPool};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{fs::File as TokioFile, io::AsyncWriteExt};
use uuid::Uuid;

pub struct ArticleService {
    article_repo: Arc<SqlxArticleRepository>,
    article_image_repo: Arc<SqlxArticleImageRepository>,
    barcode_repo: Arc<SqlxBarcodeRepository>,
    composition_repo: Arc<SqlxCompositionRepository>,
    stock_repo: Arc<SqlxStockRepository>,
    category_repo: Arc<SqlxCategoryRepository>,
}

impl ArticleService {
    pub fn new(
        article_repo: Arc<SqlxArticleRepository>,
        article_image_repo: Arc<SqlxArticleImageRepository>,
        barcode_repo: Arc<SqlxBarcodeRepository>,
        composition_repo: Arc<SqlxCompositionRepository>,
        stock_repo: Arc<SqlxStockRepository>,
        category_repo: Arc<SqlxCategoryRepository>,
    ) -> Self {
        Self {
            article_repo,
            article_image_repo,
            barcode_repo,
            composition_repo,
            stock_repo,
            category_repo,
        }
    }

    pub async fn create_article(
        &self,
        pool: &MySqlPool,
        dto: ArticleCreateRequest,
        opciona_slika: Option<ImageUploadData>, // Sliku primamo ovde kao asinhrono preuzete bajtove
    ) -> Result<u64, Error> {
        // 1. Validacija poslovnog pravila (fail-fast)
        if let (Some(comps), Some(stocks)) = (&dto.components, &dto.initial_stocks) {
            if !comps.is_empty() && !stocks.is_empty() {
                return Err(Error::Protocol(
                    "Artikal ne može istovremeno imati komponente (sastavnicu) i početne zalihe!"
                        .into(),
                ));
            }
        }

        // 2. Otvaranje transakcije i bezbedno izvlačenje konekcije
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        // 3. Obrada opcione slike pre nego što kreiramo sam artikal
        let mut finalni_image_id: Option<u64> = None;

        if let Some(slika) = opciona_slika {
            // Generišemo unikatno ime fajla na disku (npr. "uuid.png") da izbegnemo konflikte sa istim imenima
            let ekstenzija = slika.original_name.split('.').last().unwrap_or("jpg");
            let jedinstveno_ime = format!("{}.{}", Uuid::new_v4(), ekstenzija);
            let putanja_fajla = format!("./images/{}", jedinstveno_ime);

            // Asinhrono čuvanje fajla na disk pomoću Tokio FS-a
            let mut f = TokioFile::create(&putanja_fajla).await.map_err(|e| {
                Error::Protocol(format!("Neuspešno kreiranje fajla na disku: {}", e).into())
            })?;

            f.write_all(&slika.bytes).await.map_err(|e| {
                Error::Protocol(format!("Greška pri upisu bajtova slike: {}", e).into())
            })?;

            // Poziv vašeg SqlxArticleImageRepository-a sa conn: &mut MySqlConnection
            let img_id = self
                .article_image_repo
                .insert(
                    conn,
                    &jedinstveno_ime,
                    &slika.content_type,
                    &slika.original_name,
                )
                .await?;

            finalni_image_id = Some(img_id);
        }

        // 4. Rešavanje kategorije preko njene dve tabele
        let category_id = self
            .category_repo
            .resolve_from_input(&dto.category_name, conn)
            .await
            .map_err(|e| Error::Protocol(format!("Greška sa kategorijom: {}", e).into()))?;

        // Kreiramo model spreman za bazu (ID je None jer ga baza dodeljuje)
        let article = Article {
            id: None,
            name: dto.name.clone(),
            active: dto.active,
            price: dto.price.clone(), // Pravilno kloniramo Option<BigDecimal>
            admission_price_1: dto.admission_price_1.clone(),
            admission_price_2: dto.admission_price_2.clone(),
            category_id,
            article_image_id: finalni_image_id, // Prosleđujemo preračunati ID slike iz gornjeg koraka
        };

        // 5. Upis glavnog artikla i dobijanje novostvorenog ID-ja artikla
        let article_id = self.article_repo.save(&article, conn).await?;

        // 6. Masovni upis bar-kodova
        self.barcode_repo
            .save_barcodes(article_id, &dto.barcodes, conn)
            .await?;

        // 7. Ekskluzivna logika na osnovu prisustva polja
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

        // 8. Ako je sve prošlo uspešno, potvrđujemo transakciju
        tx.commit().await?;

        Ok(article_id)
    }

    pub async fn update_article(
        &self,
        pool: &MySqlPool,
        article_id: u64,
        dto: ArticleUpdateRequest, // Promenjen tip ovde
        opciona_slika: Option<ImageUploadData>,
    ) -> Result<(), Error> {
        // 1. Validacija poslovnog pravila
        if let (Some(comps), Some(stocks)) = (&dto.components, &dto.initial_stocks) {
            if !comps.is_empty() && !stocks.is_empty() {
                return Err(Error::Protocol(
                    "Artikal ne može istovremeno imati komponente i zalihe!".into(),
                ));
            }
        }

        // 2. Otvaranje transakcije i izvlačenje konekcije
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        // 3. Provera da li artikal uopšte postoji
        let article_opt = self.article_repo.find_by_id(article_id).await?;
        let postojeci_artikal = match article_opt {
            Some(art) => art,
            None => return Err(Error::RowNotFound),
        };

        let stara_slika_id = postojeci_artikal.article_image_id;
        let mut finalni_image_id: Option<u64> = None;
        let mut fajl_za_brisanje: Option<String> = None;

        // 4. Logika za sliku preko repozitorijuma (Pattern Matching)
        match (opciona_slika, stara_slika_id) {
            (Some(nova_slika), Some(stari_id)) => {
                fajl_za_brisanje = self
                    .article_image_repo
                    .find_filename_by_id(stari_id, conn)
                    .await?;

                let jedinstveno_ime = format!(
                    "{}.{}",
                    Uuid::new_v4(),
                    nova_slika.original_name.split('.').last().unwrap_or("jpg")
                );
                let mut f = TokioFile::create(format!("./images/{}", jedinstveno_ime))
                    .await
                    .map_err(|e| {
                        Error::Protocol(format!("Neuspešno kreiranje fajla: {}", e).into())
                    })?;
                f.write_all(&nova_slika.bytes).await?;

                let novi_id = self
                    .article_image_repo
                    .insert(
                        conn,
                        &jedinstveno_ime,
                        &nova_slika.content_type,
                        &nova_slika.original_name,
                    )
                    .await?;
                finalni_image_id = Some(novi_id);

                self.article_image_repo.delete(stari_id, conn).await?;
            }
            // ... (Ostala tri slučaja iz pattern matching-a ostaju identična kao pre)
            (Some(nova_slika), None) => {
                let jedinstveno_ime = format!(
                    "{}.{}",
                    Uuid::new_v4(),
                    nova_slika.original_name.split('.').last().unwrap_or("jpg")
                );
                let mut f = TokioFile::create(format!("./images/{}", jedinstveno_ime))
                    .await
                    .map_err(|e| Error::Protocol(format!("{}", e).into()))?;
                f.write_all(&nova_slika.bytes).await?;
                let novi_id = self
                    .article_image_repo
                    .insert(
                        conn,
                        &jedinstveno_ime,
                        &nova_slika.content_type,
                        &nova_slika.original_name,
                    )
                    .await?;
                finalni_image_id = Some(novi_id);
            }
            (None, Some(stari_id)) => {
                fajl_za_brisanje = self
                    .article_image_repo
                    .find_filename_by_id(stari_id, conn)
                    .await?;
                self.article_image_repo.delete(stari_id, conn).await?;
                finalni_image_id = None;
            }
            (None, None) => {
                finalni_image_id = None;
            }
        }

        // 5. Rešavanje kategorije preko njene dve tabele
        let category_id = self
            .category_repo
            .resolve_from_input(&dto.category_name, conn)
            .await
            .map_err(|e| Error::Protocol(format!("Greška sa kategorijom: {}", e).into()))?;

        let article = Article {
            id: Some(article_id),
            name: dto.name.clone(),
            active: dto.active,
            price: dto.price.clone(),
            admission_price_1: dto.admission_price_1.clone(),
            admission_price_2: dto.admission_price_2.clone(),
            category_id,
            article_image_id: finalni_image_id,
        };

        // 6. Upis artikla i pratećih repozitorijuma
        self.article_repo.save(&article, conn).await?;

        self.barcode_repo.delete_barcodes(article_id, conn).await?;
        self.barcode_repo
            .save_barcodes(article_id, &dto.barcodes, conn)
            .await?;

        // 7. Ekskluzivna logika za komponente / zalihe
        let has_components = dto.components.as_ref().map_or(false, |c| !c.is_empty());
        let has_stocks = dto.initial_stocks.as_ref().map_or(false, |s| !s.is_empty());

        if has_components {
            if let Some(components) = &dto.components {
                // Prosleđujemo referencu na komponente
                self.handle_components(article_id, components, conn).await?;
            }
        } else if has_stocks {
            if let Some(stocks) = &dto.initial_stocks {
                // Prosleđujemo referencu na zalihe
                self.handle_stock(article_id, stocks, conn).await?;
            }
        } else {
            self.stock_repo.delete_stocks(article_id, conn).await?;
            self.composition_repo
                .delete_compositions(article_id, conn)
                .await?;
        }

        // 👇 NOVO: Pozivamo automatsko čišćenje baze od neiskorišćenih kategorija
        // Izvršava se unutar iste transakcije - ako čišćenje baze baci grešku, cela izmena artikla se vraća unazad!
        self.category_repo
            .clean_unused_categories(conn)
            .await
            .map_err(|e| {
                Error::Protocol(format!("Greška prilikom čišćenja starih kategorija: {}", e).into())
            })?;

        // 8. Commit transakcije
        tx.commit().await?;

        // 9. Fizičko čišćenje hard diska
        if let Some(staro_ime_fajla) = fajl_za_brisanje {
            let putanja = format!("./images/{}", staro_ime_fajla);
            if tokio::fs::metadata(&putanja).await.is_ok() {
                let _ = tokio::fs::remove_file(putanja).await;
            }
        }

        Ok(())
    }

    async fn handle_components(
        &self,
        article_id: u64,
        components: &[ComponentInputDto], // Prima samo niz komponenti
        conn: &mut MySqlConnection,
    ) -> Result<(), Error> {
        // 1. Prvo brišemo stare komponente za ovaj artikal (ako postoje)
        self.composition_repo
            .delete_compositions(article_id, conn)
            .await?;

        // 2. Upisujemo nove komponente iz prosleđenog niza preko repozitorijuma
        // Pretpostavka je da tvoj repo ima metodu koja prihvata referencu na niz ili vektor
        self.composition_repo
            .save_compositions(article_id, components, conn)
            .await?;

        Ok(())
    }

    async fn handle_stock(
        &self,
        article_id: u64,
        stocks: &[ArticleStockInputDto], // Prima samo niz početnih zaliha
        conn: &mut MySqlConnection,
    ) -> Result<(), Error> {
        // 1. Prvo brišemo stare zalihe za ovaj artikal (ako postoje)
        self.stock_repo.delete_stocks(article_id, conn).await?;

        // 2. Upisujemo nove zalihe iz prosleđenog niza preko repozitorijuma
        self.stock_repo
            .save_stocks(article_id, stocks, conn)
            .await?;

        Ok(())
    }

    pub async fn get_all_articles(
        &self,
        query: Option<String>,
    ) -> Result<Vec<ArticleResponse>, Error> {
        // 1. Povlačimo glavne artikle na osnovu pretrage
        let articles = self.article_repo.search(query).await?;

        // Ako je baza prazna, odmah vraćamo prazan niz i prekidamo izvršavanje
        if articles.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Sakupljamo sve ID-jeve artikala da bismo masovno povukli njihove relacije
        let article_ids: Vec<u64> = articles.iter().filter_map(|a| a.id).collect();

        // 👇 NOVO: Sakupljamo unikatne ID-jeve kategorija iz artikala da bismo ih povukli odjednom
        let category_ids: Vec<u64> = articles
            .iter()
            .filter_map(|a| a.category_id) // Pretpostavka da tvoj Article entitet ima polje category_id (Option<u64>)
            .collect();

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

        // 👇 NOVO: Masovno povlačenje kategorija iz baze
        let category_rows = self
            .category_repo
            .find_categories_by_ids(&category_ids)
            .await?;

        // 4. Grupisanje bar-kodova po ID-ju artikla u memoriji
        let mut barcodes_map: HashMap<u64, Vec<String>> = HashMap::new();
        for (article_id, barcode) in barcode_rows {
            barcodes_map.entry(article_id).or_default().push(barcode);
        }

        // 5. Grupisanje zaliha po ID-ju artikla u memoriji
        let mut stocks_map: HashMap<u64, Vec<ArticleStock>> = HashMap::new();
        for stock in stock_rows {
            stocks_map.entry(stock.article_id).or_default().push(stock);
        }

        // 6. Grupisanje komponenti baze po ID-ju artikla (bundle_id) u memoriji
        let mut comps_map: HashMap<u64, Vec<ArticleComposition>> = HashMap::new();
        for comp in comp_rows {
            comps_map.entry(comp.bundle_id).or_default().push(comp);
        }

        // 👇 NOVO: Grupisanje kategorija po njihovom sopstvenom ID-ju u memoriji
        let mut categories_map: HashMap<u64, ArticleCategory> = HashMap::new();
        for cat in category_rows {
            categories_map.insert(cat.id, cat);
        }

        // 7. Sklapanje konačnih odgovora preko DTO transformacije (O(1) kompleksnost pretrage)
        let mut responses = Vec::with_capacity(articles.len());
        for article in articles {
            let id = article.id.unwrap_or_default();

            // 👇 NOVO: Uzimamo ID kategorije iz trenutnog artikla (ako ga ima)
            let art_category_id = article.category_id;

            // Izvlačimo sakupljene sirove reference iz mapa za trenutni artikal
            let art_barcodes = barcodes_map.get(&id).cloned().unwrap_or_default();
            let art_stocks = stocks_map.get(&id).cloned().unwrap_or_default();
            let art_comps = comps_map.get(&id).cloned().unwrap_or_default();

            // 👇 NOVO: Izvlačimo sklopljenu kategoriju iz mape na osnovu ID-ja kategorije artikla
            let art_category =
                art_category_id.and_then(|cat_id| categories_map.get(&cat_id).cloned());

            // Pozivamo tvoju prerađenu metodu iz dto.rs i prosleđujemo novu kategoriju
            let response = ArticleResponse::from_entity_with_barcodes(
                article,
                &art_stocks,
                &art_comps,
                art_barcodes,
                art_category, // 👈 NOVO: Prosleđujemo Option<ArticleCategory>
            );

            responses.push(response);
        }

        Ok(responses)
    }

    pub async fn check_name_availability(&self, name: &str) -> Result<bool, Error> {
        let exists = self.article_repo.check_name_exists(name).await?;
        Ok(!exists)
    }

    pub async fn find_article_by_article_name_exclude_specific(
        &self,
        name: &str,
        id: u64,
    ) -> Result<bool, Error> {
        let exists = self
            .article_repo
            .check_name_exists_exclude(name, id)
            .await?;
        Ok(!exists)
    }

    pub async fn get_affected_bundles(&self, stock_id: u64) -> Result<Vec<String>, Error> {
        let bundle_names = self.stock_repo.find_affected_bundles(stock_id).await?;
        Ok(bundle_names)
    }

    pub async fn find_by_barcode(&self, code: &str) -> Result<ArticleResponse, Error> {
        // 1. Pronalaženje ID-ja artikla preko bar-koda
        let article_id = self.barcode_repo.find_article_id_by_barcode(code).await?;

        // 2. Povlačenje samog artikla iz baze
        let article = self
            .article_repo
            .find_by_id(article_id)
            .await?
            .ok_or(Error::RowNotFound)?;

        // 3. Povlačenje svih relacija za taj artikl (bar-kodovi, zalihe, komponente)
        let barcodes = self.barcode_repo.find_by_article_ids(&[article_id]).await?;
        let stocks = self
            .stock_repo
            .find_stocks_by_article_ids(&[article_id])
            .await?;
        let compositions = self
            .composition_repo
            .find_compositions_by_bundle_ids(&[article_id])
            .await?;

        // 👇 NOVO: Povlačenje kategorije ako artikal ima dodeljen category_id
        let category = match article.category_id {
            Some(cat_id) => {
                // Koristimo funkciju find_categories_by_ids koju smo već napisali za listu,
                // i samo uzimamo prvi (i jedini) rezultat ako postoji
                self.category_repo
                    .find_categories_by_ids(&[cat_id])
                    .await?
                    .into_iter()
                    .next()
            }
            None => None,
        };

        // Čišćenje bar-kodova (ostaje isto)
        let art_barcodes: Vec<String> = barcodes.into_iter().map(|(_, b)| b).collect();

        // 4. Sklapanje konačnog odgovora sa uključenom kategorijom
        let response = ArticleResponse::from_entity_with_barcodes(
            article,
            &stocks,
            &compositions,
            art_barcodes,
            category, // 👈 NOVO: Prosleđujemo Option<ArticleCategory>
        );

        Ok(response)
    }

    pub async fn delete_article(&self, pool: &MySqlPool, article_id: u64) -> Result<(), Error> {
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        self.barcode_repo.delete_barcodes(article_id, conn).await?;
        self.stock_repo.delete_stocks(article_id, conn).await?;
        self.composition_repo
            .delete_compositions(article_id, conn)
            .await?;
        let result = self.article_repo.delete(article_id, conn).await?;

        if result.rows_affected() == 0 {
            // Ako je 0, artikal nije postojao. Ručno vraćamo RowNotFound grešku
            return Err(Error::RowNotFound);
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_all_category_names(
        &self,
        pool: &MySqlPool,
    ) -> Result<Vec<CategoryNameResponse>, Error> {
        // 1. Pokrećemo transakciju
        let mut tx = pool.begin().await?;

        // 2. Prošleđujemo mutable referencu na konekciju i koristimo `?` da odmah prosledimo eventualnu grešku
        let categories = self.category_repo.get_all_category_names(&mut *tx).await?;

        // 3. Pošto je u pitanju čist SELECT, zatvaramo transakciju
        tx.commit().await?;

        // 4. Vraćamo čist uspeh sa podacima
        Ok(categories)
    }
}
