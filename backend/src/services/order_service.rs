use crate::dto::order_dto::{
   OrderItemResponse, OrderRequest, OrderResponse, RefundItemResponse, RefundResponse,
   RevenueSummary,
};
use crate::error::ApiError;
use crate::models::{
   Article, ArticleComposition, ArticleStock, NewOrderArticle, Order, OrderArticle, OrderType,
   RefundItem,
};
use crate::repositories::{
   SqlxArticleRepository, SqlxCompositionRepository, SqlxOrderRepository, SqlxRefundRepository,
   SqlxStockRepository,
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use sqlx::MySqlPool;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub struct OrderService {
    order_repo: Arc<SqlxOrderRepository>,
    refund_repo: Arc<SqlxRefundRepository>,
    article_repo: Arc<SqlxArticleRepository>,
    composition_repo: Arc<SqlxCompositionRepository>,
    stock_repo: Arc<SqlxStockRepository>,
}

impl OrderService {
    pub fn new(
        order_repo: Arc<SqlxOrderRepository>,
        refund_repo: Arc<SqlxRefundRepository>,
        article_repo: Arc<SqlxArticleRepository>,
        composition_repo: Arc<SqlxCompositionRepository>,
        stock_repo: Arc<SqlxStockRepository>,
    ) -> Self {
        Self {
            order_repo,
            refund_repo,
            article_repo,
            composition_repo,
            stock_repo,
        }
    }

    /// Kreira porudžbinu i skida zalihu u jednoj transakciji.
    /// Cene uvek računa server (prodajna cena), frontend šalje samo artikle i količine.
    pub async fn create_order(
        &self,
        pool: &MySqlPool,
        req: OrderRequest,
    ) -> Result<OrderResponse, ApiError> {
        // 1. Spajanje duplih stavki (isti artikal skeniran dva puta), uz očuvan redosled iz korpe
        let mut stavke: Vec<(u64, u64)> = Vec::new();
        for item in &req.items {
            if item.quantity == 0 {
                return Err(ApiError::BadRequest("Količina mora biti veća od 0.".into()));
            }
            match stavke.iter_mut().find(|(id, _)| *id == item.article_id) {
                Some((_, kolicina)) => *kolicina += item.quantity,
                None => stavke.push((item.article_id, item.quantity)),
            }
        }
        if stavke.is_empty() {
            return Err(ApiError::BadRequest("Porudžbina nema nijednu stavku.".into()));
        }

        let mut tx = pool.begin().await?;
        let conn = &mut *tx;

        let ids: Vec<u64> = stavke.iter().map(|(id, _)| *id).collect();
        let artikli: HashMap<u64, Article> = self
            .article_repo
            .find_by_ids(&ids, conn)
            .await?
            .into_iter()
            .filter_map(|a| a.id.map(|id| (id, a)))
            .collect();
        if let Some((nepostojeci, _)) = stavke.iter().find(|(id, _)| !artikli.contains_key(id)) {
            return Err(ApiError::NotFound(format!(
                "Artikal #{} ne postoji.",
                nepostojeci
            )));
        }

        let mut sastavnice: HashMap<u64, Vec<ArticleComposition>> = HashMap::new();
        for c in self
            .composition_repo
            .find_compositions_by_bundle_ids(&ids)
            .await?
        {
            sastavnice.entry(c.bundle_id).or_default().push(c);
        }

        // 2. Potrebna zaliha po artiklu koji ima serije: direktna prodaja + komponente prodatih paketa.
        // BTreeMap - serije se uvek zaključavaju istim redom, pa dve istovremene prodaje ne prave deadlock.
        let mut potrebno: BTreeMap<u64, u64> = BTreeMap::new();
        for (id, kolicina) in &stavke {
            match sastavnice.get(id) {
                Some(komponente) => {
                    for k in komponente {
                        *potrebno.entry(k.component_id).or_default() += k.quantity * kolicina;
                    }
                }
                None => *potrebno.entry(*id).or_default() += kolicina,
            }
        }

        // 3. Skidanje zalihe FEFO redom (redosled određuje `lock_available_for_sale`)
        let mut raspodela: HashMap<u64, Vec<(ArticleStock, u64)>> = HashMap::new();
        for (&article_id, &trazeno) in &potrebno {
            let serije = self
                .stock_repo
                .lock_available_for_sale(article_id, conn)
                .await?;
            let dostupno: u64 = serije.iter().map(|s| s.quantity).sum();

            if dostupno < trazeno {
                // Komponenta paketa nije među artiklima iz korpe, pa joj ime tražimo posebno
                let ime = match artikli.get(&article_id) {
                    Some(a) => a.name.clone(),
                    None => self
                        .article_repo
                        .find_names_by_ids(&[article_id])
                        .await?
                        .pop()
                        .map(|(_, ime)| ime)
                        .unwrap_or_else(|| format!("#{}", article_id)),
                };
                return Err(ApiError::Conflict(format!(
                    "Nedovoljno zalihe za \"{}\": traženo {}, dostupno {}.",
                    ime, trazeno, dostupno
                )));
            }

            let mut ostatak = trazeno;
            let mut delovi = Vec::new();
            for serija in serije {
                if ostatak == 0 {
                    break;
                }
                let uzeto = ostatak.min(serija.quantity);
                let stock_id = serija.id.expect("serija učitana iz baze uvek ima id");
                self.stock_repo
                    .decrease_quantity(stock_id, uzeto, conn)
                    .await?;
                ostatak -= uzeto;
                delovi.push((serija, uzeto));
            }
            raspodela.insert(article_id, delovi);
        }

        // 4. Stavke porudžbine - za običan artikal po jedan red za svaku seriju iz koje je skinuto
        let mut redovi = Vec::new();
        let mut ukupno = BigDecimal::from(0);
        for (id, kolicina) in &stavke {
            let artikal = &artikli[id];
            ukupno += artikal.price.clone().unwrap_or_default() * BigDecimal::from(*kolicina);

            if sastavnice.contains_key(id) {
                // Paket nema sopstvene serije - zaliha je skinuta sa komponenti
                redovi.push(nova_stavka(artikal, *kolicina, None));
            } else {
                // Direktna prodaja dobija prve (FEFO) delove; ostatak su potrošile komponente paketa iz iste korpe
                let mut ostatak = *kolicina;
                for (serija, uzeto) in &raspodela[id] {
                    if ostatak == 0 {
                        break;
                    }
                    let deo = ostatak.min(*uzeto);
                    redovi.push(nova_stavka(artikal, deo, Some(serija)));
                    ostatak -= deo;
                }
            }
        }

        // Gratis: stavke čuvaju pravu cenu radi evidencije, ali porudžbina ne ulazi u prihod
        if req.is_gratis {
            ukupno = BigDecimal::from(0);
        }

        let order_id = self
            .order_repo
            .insert_order(&ukupno, req.order_type, req.is_gratis, conn)
            .await?;
        self.order_repo
            .insert_items(order_id, &redovi, conn)
            .await?;

        tx.commit().await?;

        self.find_order(order_id).await
    }

    pub async fn find_order(&self, id: u64) -> Result<OrderResponse, ApiError> {
        let order = self
            .order_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Porudžbina #{} ne postoji.", id)))?;

        Ok(self
            .sastavi_odgovore(vec![order])
            .await?
            .pop()
            .expect("jedna porudžbina daje jedan odgovor"))
    }

    pub async fn today_orders(&self) -> Result<Vec<OrderResponse>, ApiError> {
        let (pocetak, kraj) = danasnji_opseg_utc();
        let orders = self.order_repo.find_between(pocetak, kraj).await?;
        self.sastavi_odgovore(orders).await
    }

    /// Menja samo tip (sa/bez fiskalnog računa) - cene i iznosi ostaju isti
    pub async fn update_type(
        &self,
        id: u64,
        order_type: OrderType,
    ) -> Result<OrderResponse, ApiError> {
        // rows_affected je 0 i kada tip već ima tu vrednost, pa postojanje proveravamo posebno
        if self.order_repo.find_by_id(id).await?.is_none() {
            return Err(ApiError::NotFound(format!("Porudžbina #{} ne postoji.", id)));
        }
        self.order_repo.update_type(id, order_type).await?;
        self.find_order(id).await
    }

    pub async fn today_revenue(&self) -> Result<RevenueSummary, ApiError> {
        let (pocetak, kraj) = danasnji_opseg_utc();

        let mut gross_retail = BigDecimal::from(0);
        let mut gross_wholesale = BigDecimal::from(0);
        for (tip, iznos) in self.order_repo.sum_by_type_between(pocetak, kraj).await? {
            match OrderType::from_db(tip.as_deref()) {
                OrderType::Retail => gross_retail += iznos,
                OrderType::Wholesale => gross_wholesale += iznos,
            }
        }
        let total_refunds = self.refund_repo.sum_between(pocetak, kraj).await?;
        let net_revenue = &gross_retail + &gross_wholesale - &total_refunds;

        Ok(RevenueSummary {
            gross_retail,
            gross_wholesale,
            total_refunds,
            net_revenue,
        })
    }

    /// Dopunjava porudžbine stavkama i refundacijama (po jedan upit za svaku tabelu, bez N+1)
    async fn sastavi_odgovore(&self, orders: Vec<Order>) -> Result<Vec<OrderResponse>, ApiError> {
        let order_ids: Vec<u64> = orders.iter().map(|o| o.id).collect();

        let mut stavke_map: HashMap<u64, Vec<OrderArticle>> = HashMap::new();
        for s in self.order_repo.find_items_by_order_ids(&order_ids).await? {
            stavke_map.entry(s.order_id).or_default().push(s);
        }

        let refunds = self.refund_repo.find_by_order_ids(&order_ids).await?;
        let refund_ids: Vec<u64> = refunds.iter().map(|r| r.id).collect();
        let mut refund_stavke_map: HashMap<u64, Vec<RefundItem>> = HashMap::new();
        for s in self.refund_repo.find_items_by_refund_ids(&refund_ids).await? {
            if let Some(refund_id) = s.refund_id {
                refund_stavke_map.entry(refund_id).or_default().push(s);
            }
        }

        let mut refunds_map: HashMap<u64, Vec<RefundResponse>> = HashMap::new();
        for r in refunds {
            let Some(order_id) = r.original_order_id else { continue };
            let items = refund_stavke_map
                .remove(&r.id)
                .unwrap_or_default()
                .into_iter()
                .map(|s| RefundItemResponse {
                    id: s.id,
                    article_id: s.article_id,
                    article_name: s.article_name.unwrap_or_default(),
                    quantity: s.quantity.unwrap_or(0),
                })
                .collect();
            refunds_map.entry(order_id).or_default().push(RefundResponse {
                id: r.id,
                original_order_id: r.original_order_id,
                created_at: r.created_at.map(u_utc),
                total_amount: r.total_amount.unwrap_or_default(),
                items,
            });
        }

        Ok(orders
            .into_iter()
            .map(|o| OrderResponse {
                id: o.id,
                order_type: OrderType::from_db(o.order_type.as_deref()),
                created_at: o.created_at.map(u_utc),
                total_amount: o.total_amount.unwrap_or_default(),
                is_gratis: o.is_gratis,
                items: stavke_map
                    .remove(&o.id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| OrderItemResponse {
                        id: s.id,
                        article_id: s.article_id,
                        article_name: s.article_name_snapshot,
                        price_at_sale: s.price_at_sale,
                        quantity: s.quantity,
                        refunded_quantity: s.refunded_quantity,
                        available_to_refund: s.quantity - s.refunded_quantity,
                    })
                    .collect(),
                refunds: refunds_map.remove(&o.id).unwrap_or_default(),
            })
            .collect())
    }
}

fn nova_stavka(artikal: &Article, kolicina: u64, serija: Option<&ArticleStock>) -> NewOrderArticle {
    NewOrderArticle {
        article_id: artikal.id.expect("artikal učitan iz baze uvek ima id"),
        article_name: artikal.name.clone(),
        price_at_sale: artikal.price.clone().unwrap_or_default(),
        admission_price_at_sale: artikal.admission_price_1.clone().unwrap_or_default(),
        admission_price2_at_sale: artikal.admission_price_2.clone(),
        quantity: kolicina,
        batch_number: serija.and_then(|s| s.batch_number.clone()),
        expiration_date: serija.and_then(|s| s.expiration_date),
    }
}

/// Vremena se u bazi čuvaju u UTC-u, a "danas" je lokalni kalendarski dan kase
fn danasnji_opseg_utc() -> (NaiveDateTime, NaiveDateTime) {
    let danas = Local::now().date_naive();
    let ponoc_u_utc = |dan: NaiveDate| {
        dan.and_hms_opt(0, 0, 0)
            .expect("ponoć je validno vreme")
            .and_local_timezone(Local)
            .earliest() // Prelazak na letnje/zimsko vreme kod nas nije u ponoć, ali za svaki slučaj
            .expect("lokalna ponoć postoji")
            .naive_utc()
    };
    let sutra = danas.succ_opt().expect("sutrašnji datum postoji");
    (ponoc_u_utc(danas), ponoc_u_utc(sutra))
}

fn u_utc(vreme: NaiveDateTime) -> DateTime<Utc> {
    Utc.from_utc_datetime(&vreme)
}
