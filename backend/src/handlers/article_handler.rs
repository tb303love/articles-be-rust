use crate::context::AppContext;
use crate::dto::{ArticleCreateRequest, ArticleUpdateRequest};
use crate::extractors::article_extractor::ArticleUpload;
use actix_web::web::patch;
use actix_web::{
   web::{self, delete, get, post, Data, Path, Query},
   HttpResponse,
};
use serde::Deserialize;
use sqlx::Error;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: Option<String>,
}

#[derive(Deserialize)]
pub struct NameCheckQuery {
    pub name: String,
}

#[derive(Deserialize)]
pub struct NameExcludeQuery {
    pub name: String,
    pub id: u64,
}

pub fn rute(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/articles")
            .route("", get().to(get_all_articles))
            .route("", post().to(create_article))
            .route("/count", get().to(count_articles))
            .route("/check-name", get().to(check_name))
            .route(
                "/check-name-exclude-current",
                get().to(check_name_exclude_current),
            )
            .route(
                "/affected-bundles/{stock_id}",
                get().to(find_affected_bundles),
            )
            .route("/barcode/{code}", get().to(find_by_barcode))
            .route("/categories", get().to(get_all_category_names))
            .route("/{id}", patch().to(update_article))
            .route("/{id}", delete().to(delete_article)),
    );
}

// --- HANDLERI (ČISTE FUNKCIJE SA EKSPLICITNIM IZVLAČENJEM) ---

async fn get_all_articles(ctx: Data<AppContext>, query: Query<SearchQuery>) -> HttpResponse {
    let service = ctx.article_service();
    match service.get_all_articles(query.into_inner().query).await {
        Ok(articles) => HttpResponse::Ok().json(articles),
        Err(e) => {
            log::error!("Greška pri čitanju artikala iz baze: {:?}", e);
            HttpResponse::InternalServerError().json("Greška na serveru")
        }
    }
}

async fn count_articles(ctx: Data<AppContext>) -> HttpResponse {
    let service = ctx.article_service();
    match service.count_articles().await {
        Ok(count) => HttpResponse::Ok().json(count),
        Err(e) => {
            log::error!("Greška pri brojanju artikala: {:?}", e);
            HttpResponse::InternalServerError().json("Greška na serveru")
        }
    }
}

async fn check_name(ctx: Data<AppContext>, query: Query<NameCheckQuery>) -> HttpResponse {
    let service = ctx.article_service();
    match service.check_name_availability(&query.name).await {
        Ok(is_available) => HttpResponse::Ok().json(is_available),
        Err(e) => {
            log::error!("Greška pri proveri imena artikla: {:?}", e);
            HttpResponse::InternalServerError().json("Greška na serveru")
        }
    }
}

async fn check_name_exclude_current(
    ctx: Data<AppContext>,
    query: Query<NameExcludeQuery>,
) -> HttpResponse {
    let service = ctx.article_service();
    let q = query.into_inner();
    match service
        .find_article_by_article_name_exclude_specific(&q.name, q.id)
        .await
    {
        Ok(is_available) => HttpResponse::Ok().json(is_available),
        Err(e) => {
            log::error!("Greška pri proveri izuzetog imena artikla: {:?}", e);
            HttpResponse::InternalServerError().json("Greška na serveru")
        }
    }
}

async fn find_affected_bundles(ctx: Data<AppContext>, path: Path<u64>) -> HttpResponse {
    let service = ctx.article_service();
    let stock_id = path.into_inner();
    match service.get_affected_bundles(stock_id).await {
        Ok(bundle_names) => HttpResponse::Ok().json(bundle_names),
        Err(e) => {
            log::error!(
                "Greška pri pronalaženju pogođenih paketa za ID {}: {:?}",
                stock_id,
                e
            );
            HttpResponse::InternalServerError().json("Greška na serveru")
        }
    }
}

async fn find_by_barcode(ctx: Data<AppContext>, path: Path<String>) -> HttpResponse {
    let service = ctx.article_service();
    let code = path.into_inner();
    match service.find_by_barcode(&code).await {
        Ok(article) => HttpResponse::Ok().json(article),
        Err(Error::RowNotFound) => HttpResponse::NotFound().json("Artikal nije pronađen"),
        Err(e) => {
            log::error!(
                "Greška pri pronalaženju artikla preko barkoda {}: {:?}",
                code,
                e
            );
            HttpResponse::InternalServerError().json("Greška na serveru")
        }
    }
}

async fn create_article(
    ctx: Data<AppContext>,
    upload: ArticleUpload<ArticleCreateRequest>, // Magija: Actix sve završava u pozadini!
) -> HttpResponse {
    let service = ctx.article_service();

    // Podaci su čisti i bezbedni za direktno slanje u servis
    match service
        .create_article(ctx.pool(), upload.dto, upload.slika)
        .await
    {
        Ok(new_id) => HttpResponse::Created().json(new_id),
        Err(e) => {
            log::error!("Greška pri kreiranju artikla u servisu: {:?}", e);
            HttpResponse::InternalServerError().json("Greška na serveru")
        }
    }
}

async fn update_article(
    ctx: Data<AppContext>,
    path: Path<u64>,
    upload: ArticleUpload<ArticleUpdateRequest>, // Isti ekstraktor, drugi DTO model!
) -> HttpResponse {
    let service = ctx.article_service();
    let article_id = path.into_inner();

    match service
        .update_article(ctx.pool(), article_id, upload.dto, upload.slika)
        .await
    {
        Ok(_) => HttpResponse::Ok().json("Artikal uspešno ažuriran"),
        Err(err) => match err {
            Error::RowNotFound => HttpResponse::NotFound().json("Artikal nije pronađen"),
            Error::Protocol(msg) => HttpResponse::BadRequest().json(msg),
            _ => {
                log::error!("Greška pri ažuriranju artikla: {:?}", err);
                HttpResponse::InternalServerError().json("Greška na serveru")
            }
        },
    }
}

async fn delete_article(ctx: Data<AppContext>, path: Path<u64>) -> HttpResponse {
    let service = ctx.article_service();
    let article_id = path.into_inner();

    match service.delete_article(ctx.pool(), article_id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => match err {
            Error::RowNotFound => HttpResponse::NotFound().json("Artikal nije pronađen"),
            _ => {
                log::error!("Greška pri brisanju artikla: {:?}", err);
                HttpResponse::InternalServerError().json("Greška na serveru")
            }
        },
    }
}

async fn get_all_category_names(ctx: Data<AppContext>) -> HttpResponse {
    let service = ctx.article_service();
    match service.get_all_category_names(ctx.pool()).await {
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(e) => {
            log::error!("Greška pri čitanju kategorije artikala iz baze: {:?}", e);
            HttpResponse::InternalServerError().json("Greška na serveru")
        }
    }
}
