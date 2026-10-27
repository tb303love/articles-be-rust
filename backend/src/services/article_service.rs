use crate::dto::{
   ArticleCategory, ArticleCreateRequest, ArticleRelations, ArticleResponse,
   ArticleStockInputDto, ArticleUpdateRequest, BulkImportItem, BulkImportResponse,
   CategoryNameResponse, ComponentInputDto, ComponentLookup,
};
use crate::error::ApiError;
use crate::models::{Article, ArticleComposition, ArticleImage, ArticleStock, ImageUploadData};
use crate::repositories::{
   SqlxArticleImageRepository, SqlxArticleRepository, SqlxBarcodeRepository,
   SqlxCategoryRepository, SqlxCompositionRepository, SqlxStockRepository,
   SqlxWriteOffRepository,
};
use bigdecimal::BigDecimal;
use sqlx::{MySqlConnection, MySqlPool};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::{fs::File as TokioFile, io::AsyncWriteExt};
use uuid::Uuid;

const FOLDER_SLIKA: &str = "./images";

pub struct ArticleService {
    article_repo: Arc<SqlxArticleRepository>,
    article_image_repo: Arc<SqlxArticleImageRepository>,
    barcode_repo: Arc<SqlxBarcodeRepository>,
    composition_repo: Arc<SqlxCompositionRepository>,
    stock_repo: Arc<SqlxStockRepository>,
    category_repo: Arc<SqlxCategoryRepository>,
    write_off_repo: Arc<SqlxWriteOffRepository>,
}

impl ArticleService {
    pub fn new(
        article_repo: Arc<SqlxArticleRepository>,
        article_image_repo: Arc<SqlxArticleImageRepository>,
        barcode_repo: Arc<SqlxBarcodeRepository>,
        composition_repo: Arc<SqlxCompositionRepository>,
        stock_repo: Arc<SqlxStockRepository>,
        category_repo: Arc<SqlxCategoryRepository>,
        write_off_repo: Arc<SqlxWriteOffRepository>,
    ) -> Self {
        Self {
            article_repo,
            article_image_repo,
            barcode_repo,
            composition_repo,
            stock_repo,
            category_repo,
            write_off_repo,
        }
    }

    pub async fn create_article(
        &self,
        pool: &MySqlPool,
        dto: ArticleCreateRequest,
        opciona_slika: Option<ImageUploadData>, // Sliku primamo ovde kao asinhrono preuzete bajtove
    ) -> Result<u64, ApiError> {
        // 1. Validacija poslovnih pravila (fail-fast)
        validiraj_artikal(
            &dto.name,
            [&dto.price, &dto.admission_price_1, &dto.admission_price_2],
            &dto.components,
            &dto.initial_stocks,
            None,
        )?;

        // 2. Otvaranje transakcije i bezbedno izvlačenje konekcije
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        // 3. Red za sliku se upisuje odmah (artiklu treba njen ID), a sam fajl tek pred commit
        let mut finalni_image_id: Option<u64> = None;
        let mut nova_slika: Option<(String, Vec<u8>)> = None;

        if let Some(slika) = opciona_slika {
            let jedinstveno_ime = novo_ime_slike(&slika.original_name);
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
            nova_slika = Some((jedinstveno_ime, slika.bytes));
        }

        // 4. Rešavanje kategorije preko njene dve tabele
        let category_id = self
            .category_repo
            .resolve_from_input(&dto.category_name, conn)
            .await?;

        // Kreiramo model spreman za bazu (ID je None jer ga baza dodeljuje)
        let article = Article {
            id: None,
            name: dto.name.trim().to_string(),
            active: dto.active,
            price: dto.price.clone(), // Pravilno kloniramo Option<BigDecimal>
            admission_price_1: dto.admission_price_1.clone(),
            admission_price_2: dto.admission_price_2.clone(),
            category_id,
            article_image_id: finalni_image_id, // Prosleđujemo preračunati ID slike iz gornjeg koraka
        };

        // 5. Upis glavnog artikla - duplo ime ovde postaje 409 (UNIQUE uk_article_name)
        let article_id = self.article_repo.save(&article, conn).await?;

        // 6. Masovni upis bar-kodova - barkod drugog artikla ovde postaje 409
        self.barcode_repo
            .save_barcodes(article_id, &dto.barcodes, conn)
            .await?;

        // 7. Ekskluzivna logika na osnovu prisustva polja
        let has_components = dto.components.as_ref().map_or(false, |c| !c.is_empty());
        let has_stocks = dto.initial_stocks.as_ref().map_or(false, |s| !s.is_empty());

        if has_components {
            if let Some(components) = &dto.components {
                // Nepostojeća komponenta ovde postaje 400 (FK)
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

        // 8. Fajl na disk tek kada su svi upisi u bazu prošli, pa commit
        self.potvrdi_sa_slikom(tx, nova_slika.as_ref()).await?;

        Ok(article_id)
    }

    pub async fn update_article(
        &self,
        pool: &MySqlPool,
        article_id: u64,
        dto: ArticleUpdateRequest, // Promenjen tip ovde
        opciona_slika: Option<ImageUploadData>,
    ) -> Result<(), ApiError> {
        // 1. Validacija poslovnih pravila
        validiraj_artikal(
            &dto.name,
            [&dto.price, &dto.admission_price_1, &dto.admission_price_2],
            &dto.components,
            &dto.initial_stocks,
            Some(article_id),
        )?;

        // 2. Otvaranje transakcije i izvlačenje konekcije
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        // 3. Provera da li artikal uopšte postoji
        let postojeci_artikal = self
            .article_repo
            .find_by_id(article_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Artikal #{} ne postoji.", article_id)))?;

        let stara_slika_id = postojeci_artikal.article_image_id;
        let mut finalni_image_id: Option<u64> = None;
        let mut fajl_za_brisanje: Option<String> = None;
        let mut nova_slika_na_disk: Option<(String, Vec<u8>)> = None;

        // 4. Nova slika (ako je stigla) zamenjuje staru; bez nove slike stara se uklanja
        if let Some(stari_id) = stara_slika_id {
            fajl_za_brisanje = self
                .article_image_repo
                .find_filename_by_id(stari_id, conn)
                .await?;
        }

        if let Some(nova_slika) = opciona_slika {
            let jedinstveno_ime = novo_ime_slike(&nova_slika.original_name);
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
            nova_slika_na_disk = Some((jedinstveno_ime, nova_slika.bytes));
        }

        // 5. Rešavanje kategorije preko njene dve tabele
        let category_id = self
            .category_repo
            .resolve_from_input(&dto.category_name, conn)
            .await?;

        let article = Article {
            id: Some(article_id),
            name: dto.name.trim().to_string(),
            active: dto.active,
            price: dto.price.clone(),
            admission_price_1: dto.admission_price_1.clone(),
            admission_price_2: dto.admission_price_2.clone(),
            category_id,
            article_image_id: finalni_image_id,
        };

        // 6. Upis artikla i pratećih repozitorijuma
        self.article_repo.save(&article, conn).await?;

        // Stari red slike briše se tek kada artikal više ne pokazuje na njega
        if let Some(stari_id) = stara_slika_id {
            self.article_image_repo.delete(stari_id, conn).await?;
        }

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

        // Automatsko čišćenje neiskorišćenih kategorija u istoj transakciji -
        // ako čišćenje baci grešku, cela izmena artikla se vraća unazad
        self.category_repo.clean_unused_categories(conn).await?;

        // 8. Nova slika na disk, pa commit
        self.potvrdi_sa_slikom(tx, nova_slika_na_disk.as_ref()).await?;

        // 9. Fizičko čišćenje hard diska - stara slika tek posle uspešnog commit-a
        if let Some(staro_ime_fajla) = fajl_za_brisanje {
            obrisi_sliku(&staro_ime_fajla).await;
        }

        Ok(())
    }

    /// Upisuje novu sliku na disk i potvrđuje transakciju.
    /// Ako upis fajla ili commit ne uspe, fajl se briše - na disku ne ostaju slike bez artikla.
    async fn potvrdi_sa_slikom(
        &self,
        tx: sqlx::Transaction<'_, sqlx::MySql>,
        slika: Option<&(String, Vec<u8>)>,
    ) -> Result<(), ApiError> {
        if let Some((ime, bytes)) = slika {
            if let Err(e) = upisi_sliku(ime, bytes).await {
                obrisi_sliku(ime).await; // Delimično upisan fajl
                return Err(e); // tx se odbacuje (rollback) kada izađe iz opsega
            }
        }

        if let Err(e) = tx.commit().await {
            if let Some((ime, _)) = slika {
                obrisi_sliku(ime).await;
            }
            return Err(e.into());
        }
        Ok(())
    }

    async fn handle_components(
        &self,
        article_id: u64,
        components: &[ComponentInputDto], // Prima samo niz komponenti
        conn: &mut MySqlConnection,
    ) -> Result<(), ApiError> {
        // 1. Prvo brišemo stare komponente za ovaj artikal (ako postoje)
        self.composition_repo
            .delete_compositions(article_id, conn)
            .await?;

        // 2. Upisujemo nove komponente iz prosleđenog niza preko repozitorijuma
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
    ) -> Result<(), ApiError> {
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
    ) -> Result<Vec<ArticleResponse>, ApiError> {
        // 1. Povlačimo glavne artikle na osnovu pretrage
        let articles = self.article_repo.search(query).await?;

        // Ako je baza prazna, odmah vraćamo prazan niz i prekidamo izvršavanje
        if articles.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Sakupljamo sve ID-jeve artikala da bismo masovno povukli njihove relacije
        let article_ids: Vec<u64> = articles.iter().filter_map(|a| a.id).collect();

        // Sakupljamo ID-jeve kategorija iz artikala da bismo ih povukli odjednom
        let category_ids: Vec<u64> = articles.iter().filter_map(|a| a.category_id).collect();

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

        // Masovno povlačenje kategorija iz baze
        let category_rows = self
            .category_repo
            .find_categories_by_ids(&category_ids)
            .await?;

        // Masovno povlačenje slika
        let image_ids: Vec<u64> = articles.iter().filter_map(|a| a.article_image_id).collect();
        let image_rows = self.article_image_repo.find_by_ids(&image_ids).await?;

        // Imena i zalihe komponenti iz sastavnica (komponente su i same artikli)
        let component_lookup = self.load_component_lookup(&comp_rows).await?;

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

        // Grupisanje kategorija po njihovom sopstvenom ID-ju u memoriji
        let mut categories_map: HashMap<u64, ArticleCategory> = HashMap::new();
        for cat in category_rows {
            categories_map.insert(cat.id, cat);
        }

        let mut images_map: HashMap<u64, ArticleImage> =
            image_rows.into_iter().map(|img| (img.id, img)).collect();

        // 7. Sklapanje konačnih odgovora preko DTO transformacije (O(1) kompleksnost pretrage)
        let mut responses = Vec::with_capacity(articles.len());
        for article in articles {
            let id = article.id.expect("artikal učitan iz baze uvek ima id");

            // Svaki artikal se pojavljuje samo jednom, pa podatke premeštamo iz mapa umesto kloniranja.
            // Kategorije su deljene između artikala, zato se one jedine kloniraju.
            let rel = ArticleRelations {
                barcodes: barcodes_map.remove(&id).unwrap_or_default(),
                stocks: stocks_map.remove(&id).unwrap_or_default(),
                components: comps_map.remove(&id).unwrap_or_default(),
                category: article
                    .category_id
                    .and_then(|cat_id| categories_map.get(&cat_id).cloned()),
                image: article
                    .article_image_id
                    .and_then(|img_id| images_map.remove(&img_id)),
            };

            responses.push(ArticleResponse::from_parts(article, rel, &component_lookup));
        }

        Ok(responses)
    }

    /// Ukupan broj artikala, nezavisno od pretrage
    pub async fn count_articles(&self) -> Result<u64, ApiError> {
        Ok(self.article_repo.count_all().await?)
    }

    pub async fn check_name_availability(&self, name: &str) -> Result<bool, ApiError> {
        let exists = self.article_repo.check_name_exists(name.trim()).await?;
        Ok(!exists)
    }

    pub async fn find_article_by_article_name_exclude_specific(
        &self,
        name: &str,
        id: u64,
    ) -> Result<bool, ApiError> {
        let exists = self
            .article_repo
            .check_name_exists_exclude(name.trim(), id)
            .await?;
        Ok(!exists)
    }

    pub async fn get_affected_bundles(&self, stock_id: u64) -> Result<Vec<String>, ApiError> {
        Ok(self.stock_repo.find_affected_bundles(stock_id).await?)
    }

    /// Uvoz iz Excel-a u jednoj transakciji - ili prođu svi redovi, ili nijedan.
    /// Postojeći artikal (po imenu) dobija nove cene i kategoriju, a zaliha se dodaje kao nova serija.
    pub async fn bulk_import(
        &self,
        pool: &MySqlPool,
        items: Vec<BulkImportItem>,
    ) -> Result<BulkImportResponse, ApiError> {
        if items.is_empty() {
            return Err(ApiError::BadRequest("Nema artikala za uvoz.".into()));
        }

        // Ceo fajl se proverava pre upisa - greška u jednom redu odbija uvoz sa jasnom porukom
        for (indeks, item) in items.iter().enumerate() {
            let oznaka = if item.name.trim().is_empty() {
                format!("red {}", indeks + 1)
            } else {
                format!("\"{}\"", item.name.trim())
            };
            proveri_cene(
                [&item.price, &item.admission_price_1, &item.admission_price_2],
                &oznaka,
            )?;
        }

        let mut tx = pool.begin().await?;
        let conn = &mut *tx;
        let mut result = BulkImportResponse::default();

        for item in items {
            let name = item.name.trim();
            if name.is_empty() {
                continue; // Worker već odbacuje redove bez imena, ovo je samo zaštita
            }

            let category_id = self
                .category_repo
                .resolve_from_input(&item.category, conn)
                .await?;

            // Pretraga ide kroz istu konekciju, pa se vidi i artikal upisan ranije u ovom fajlu (duplikat imena)
            let article_id = match self.article_repo.find_by_name(name, conn).await? {
                Some(mut article) => {
                    article.price = item.price;
                    article.admission_price_1 = item.admission_price_1;
                    article.admission_price_2 = item.admission_price_2;
                    // Prazna kolona "Tip" ne briše postojeću kategoriju
                    article.category_id = category_id.or(article.category_id);
                    result.updated += 1;
                    self.article_repo.save(&article, conn).await?
                }
                None => {
                    let article = Article {
                        id: None,
                        name: name.to_string(),
                        active: true,
                        price: item.price,
                        admission_price_1: item.admission_price_1,
                        admission_price_2: item.admission_price_2,
                        category_id,
                        article_image_id: None,
                    };
                    result.created += 1;
                    self.article_repo.save(&article, conn).await?
                }
            };

            let quantity = item.initial_stock.unwrap_or(0);
            if quantity > 0 {
                if self.composition_repo.is_bundle(article_id, conn).await? {
                    result.skipped_bundle_stock += 1;
                } else {
                    let stock = ArticleStockInputDto {
                        quantity: quantity as u64,
                        expiration_date: None,
                        batch_number: None,
                    };
                    self.stock_repo
                        .save_stocks(article_id, std::slice::from_ref(&stock), conn)
                        .await?;
                }
            }
        }

        // Izmenjeni artikli su mogli da napuste svoje stare kategorije
        self.category_repo.clean_unused_categories(conn).await?;

        tx.commit().await?;
        Ok(result)
    }

    /// Otpisuje celu seriju: upisuje otpis po nabavnoj ceni i spušta količinu serije na 0
    pub async fn write_off_stock(
        &self,
        pool: &MySqlPool,
        stock_id: u64,
        user_id: u64,
    ) -> Result<(), ApiError> {
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        let stock = self
            .stock_repo
            .lock_for_write_off(stock_id, conn)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Serija #{} ne postoji.", stock_id)))?;

        if stock.quantity == 0 {
            return Err(ApiError::Conflict("Serija je već otpisana.".into()));
        }

        let ukupna_vrednost = &stock.nabavna_cena * BigDecimal::from(stock.quantity);
        let napomena = match &stock.batch_number {
            Some(serija) => format!("Otpis serije {} ({} kom)", serija, stock.quantity),
            None => format!("Otpis serije #{} ({} kom)", stock_id, stock.quantity),
        };

        let write_off_id = self
            .write_off_repo
            .create(Some(user_id), Some(&napomena), &ukupna_vrednost, conn)
            .await?;
        self.write_off_repo
            .add_item(write_off_id, &stock, conn)
            .await?;
        self.stock_repo.clear_quantity(stock_id, conn).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn find_by_barcode(&self, code: &str) -> Result<ArticleResponse, ApiError> {
        let ne_postoji = || ApiError::NotFound(format!("Artikal sa barkodom {} ne postoji.", code));

        // 1. Pronalaženje ID-ja artikla preko bar-koda
        let article_id = match self.barcode_repo.find_article_id_by_barcode(code).await {
            Ok(id) => id,
            Err(sqlx::Error::RowNotFound) => return Err(ne_postoji()),
            Err(e) => return Err(e.into()),
        };

        // 2. Povlačenje samog artikla iz baze
        let article = self
            .article_repo
            .find_by_id(article_id)
            .await?
            .ok_or_else(ne_postoji)?;

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

        // Kategorija preko iste funkcije kao za listu - uzimamo prvi (i jedini) rezultat
        let category = match article.category_id {
            Some(cat_id) => self
                .category_repo
                .find_categories_by_ids(&[cat_id])
                .await?
                .into_iter()
                .next(),
            None => None,
        };

        let image = match article.article_image_id {
            Some(img_id) => self
                .article_image_repo
                .find_by_ids(&[img_id])
                .await?
                .into_iter()
                .next(),
            None => None,
        };

        let component_lookup = self.load_component_lookup(&compositions).await?;

        // 4. Sklapanje konačnog odgovora sa svim relacijama
        let rel = ArticleRelations {
            barcodes: barcodes.into_iter().map(|(_, b)| b).collect(),
            stocks,
            components: compositions,
            category,
            image,
        };

        Ok(ArticleResponse::from_parts(article, rel, &component_lookup))
    }

    /// Povlači imena i ukupne zalihe svih artikala koji se pojavljuju kao komponente u datim sastavnicama
    async fn load_component_lookup(
        &self,
        compositions: &[ArticleComposition],
    ) -> Result<ComponentLookup, ApiError> {
        let mut component_ids: Vec<u64> = compositions.iter().map(|c| c.component_id).collect();
        component_ids.sort_unstable();
        component_ids.dedup();

        if component_ids.is_empty() {
            return Ok(ComponentLookup::default());
        }

        let names = self
            .article_repo
            .find_names_by_ids(&component_ids)
            .await?
            .into_iter()
            .collect();

        let mut stock: HashMap<u64, u64> = HashMap::new();
        for s in self
            .stock_repo
            .find_stocks_by_article_ids(&component_ids)
            .await?
        {
            *stock.entry(s.article_id).or_default() += s.quantity;
        }

        Ok(ComponentLookup { names, stock })
    }

    pub async fn delete_article(&self, pool: &MySqlPool, article_id: u64) -> Result<(), ApiError> {
        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        let artikal = self
            .article_repo
            .find_by_id(article_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Artikal #{} ne postoji.", article_id)))?;

        // FK bi tiho izbacio artikal iz paketa (CASCADE) i paket bi ostao neispravan
        let paketi = self
            .composition_repo
            .find_bundle_names_using(article_id, conn)
            .await?;
        if !paketi.is_empty() {
            return Err(ApiError::Conflict(format!(
                "Artikal \"{}\" je komponenta paketa: {}. Prvo ga uklonite iz tih paketa.",
                artikal.name,
                paketi.join(", ")
            )));
        }

        let fajl_slike = match artikal.article_image_id {
            Some(img_id) => {
                self.article_image_repo
                    .find_filename_by_id(img_id, conn)
                    .await?
            }
            None => None,
        };

        self.barcode_repo.delete_barcodes(article_id, conn).await?;
        self.stock_repo.delete_stocks(article_id, conn).await?;
        self.composition_repo
            .delete_compositions(article_id, conn)
            .await?;
        let result = self.article_repo.delete(article_id, conn).await?;

        if result.rows_affected() == 0 {
            // Neko ga je obrisao u međuvremenu
            return Err(ApiError::NotFound(format!("Artikal #{} ne postoji.", article_id)));
        }

        if let Some(img_id) = artikal.article_image_id {
            self.article_image_repo.delete(img_id, conn).await?;
        }
        self.category_repo.clean_unused_categories(conn).await?;

        tx.commit().await?;

        if let Some(ime) = fajl_slike {
            obrisi_sliku(&ime).await;
        }
        Ok(())
    }

    pub async fn get_all_category_names(
        &self,
        pool: &MySqlPool,
    ) -> Result<Vec<CategoryNameResponse>, ApiError> {
        let mut conn = pool.acquire().await?;
        Ok(self.category_repo.get_all_category_names(&mut conn).await?)
    }
}

// --- VALIDACIJA ---

/// Zajednička provera za kreiranje i izmenu artikla (`article_id` je Some pri izmeni)
fn validiraj_artikal(
    name: &str,
    cene: [&Option<BigDecimal>; 3],
    components: &Option<Vec<ComponentInputDto>>,
    stocks: &Option<Vec<ArticleStockInputDto>>,
    article_id: Option<u64>,
) -> Result<(), ApiError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("Ime artikla je obavezno.".into()));
    }
    proveri_cene(cene, &format!("\"{}\"", name))?;

    let components = components.as_deref().unwrap_or_default();
    let stocks = stocks.as_deref().unwrap_or_default();

    if !components.is_empty() && !stocks.is_empty() {
        return Err(ApiError::BadRequest(
            "Artikal ne može istovremeno imati komponente (sastavnicu) i zalihe.".into(),
        ));
    }

    let mut vidjene = HashSet::new();
    for c in components {
        if c.quantity == 0 {
            return Err(ApiError::BadRequest(
                "Količina komponente u paketu mora biti veća od 0.".into(),
            ));
        }
        if Some(c.component_id) == article_id {
            return Err(ApiError::BadRequest("Paket ne može sadržati sam sebe.".into()));
        }
        if !vidjene.insert(c.component_id) {
            return Err(ApiError::BadRequest(format!(
                "Komponenta #{} je navedena više puta.",
                c.component_id
            )));
        }
    }

    Ok(())
}

/// Cene su opcione, ali ne smeju biti negativne
fn proveri_cene(cene: [&Option<BigDecimal>; 3], oznaka: &str) -> Result<(), ApiError> {
    const NAZIVI: [&str; 3] = ["Prodajna cena", "Ulazna cena 1", "Ulazna cena 2"];
    let nula = BigDecimal::from(0);
    for (cena, naziv) in cene.iter().zip(NAZIVI) {
        if let Some(c) = cena {
            if *c < nula {
                return Err(ApiError::BadRequest(format!(
                    "{} za {} ne može biti negativna.",
                    naziv, oznaka
                )));
            }
        }
    }
    Ok(())
}

// --- SLIKE NA DISKU ---

/// Unikatno ime fajla na disku (npr. "uuid.png") da izbegnemo konflikte sa istim imenima
fn novo_ime_slike(original_name: &str) -> String {
    let ekstenzija = original_name.rsplit('.').next().unwrap_or("jpg");
    format!("{}.{}", Uuid::new_v4(), ekstenzija)
}

async fn upisi_sliku(ime: &str, bytes: &[u8]) -> Result<(), ApiError> {
    tokio::fs::create_dir_all(FOLDER_SLIKA).await?;
    let mut f = TokioFile::create(format!("{}/{}", FOLDER_SLIKA, ime)).await?;
    f.write_all(bytes).await?;
    Ok(())
}

/// Brisanje je "najbolji pokušaj" - neobrisan fajl ne sme da obori već uspešnu operaciju
async fn obrisi_sliku(ime: &str) {
    let putanja = format!("{}/{}", FOLDER_SLIKA, ime);
    match tokio::fs::remove_file(&putanja).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("Slika {} nije obrisana sa diska: {:?}", putanja, e),
    }
}
