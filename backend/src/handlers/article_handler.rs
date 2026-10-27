use crate::context::AppContext;
use crate::dto::{ArticleCreateRequest, ArticleUpdateRequest, BulkImportItem};
use crate::error::ApiError;
use crate::extractors::article_extractor::ArticleUpload;
use crate::utils::auth_middleware::Claims;
use actix_web::web::patch;
use actix_web::{
   web::{self, delete, get, post, Data, Json, Path, Query, ReqData},
   HttpResponse,
};
use serde::Deserialize;

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
            .route("/bulk-import", post().to(bulk_import))
            .route(
                "/stock/{stock_id}/affected-bundles",
                get().to(find_affected_bundles),
            )
            .route("/stock/{stock_id}", delete().to(write_off_stock))
            .route("/barcode/{code}", get().to(find_by_barcode))
            .route("/categories", get().to(get_all_category_names))
            .route("/{id}", patch().to(update_article))
            .route("/{id}", delete().to(delete_article)),
    );
}

// --- HANDLERI (ČISTE FUNKCIJE SA EKSPLICITNIM IZVLAČENJEM) ---
// Greške pretvara ApiError (status + `{ "message" }`), handleri opisuju samo uspešan ishod

async fn get_all_articles(
    ctx: Data<AppContext>,
    query: Query<SearchQuery>,
) -> Result<HttpResponse, ApiError> {
    let articles = ctx
        .article_service()
        .get_all_articles(query.into_inner().query)
        .await?;
    Ok(HttpResponse::Ok().json(articles))
}

async fn count_articles(ctx: Data<AppContext>) -> Result<HttpResponse, ApiError> {
    let count = ctx.article_service().count_articles().await?;
    Ok(HttpResponse::Ok().json(count))
}

async fn check_name(
    ctx: Data<AppContext>,
    query: Query<NameCheckQuery>,
) -> Result<HttpResponse, ApiError> {
    let is_available = ctx
        .article_service()
        .check_name_availability(&query.name)
        .await?;
    Ok(HttpResponse::Ok().json(is_available))
}

async fn check_name_exclude_current(
    ctx: Data<AppContext>,
    query: Query<NameExcludeQuery>,
) -> Result<HttpResponse, ApiError> {
    let q = query.into_inner();
    let is_available = ctx
        .article_service()
        .find_article_by_article_name_exclude_specific(&q.name, q.id)
        .await?;
    Ok(HttpResponse::Ok().json(is_available))
}

async fn find_affected_bundles(
    ctx: Data<AppContext>,
    path: Path<u64>,
) -> Result<HttpResponse, ApiError> {
    let bundle_names = ctx
        .article_service()
        .get_affected_bundles(path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(bundle_names))
}

async fn bulk_import(
    ctx: Data<AppContext>,
    items: Json<Vec<BulkImportItem>>,
) -> Result<HttpResponse, ApiError> {
    let result = ctx
        .article_service()
        .bulk_import(ctx.pool(), items.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

async fn write_off_stock(
    ctx: Data<AppContext>,
    path: Path<u64>,
    claims: ReqData<Claims>, // Ubacuje ga Authentication middleware
) -> Result<HttpResponse, ApiError> {
    ctx.article_service()
        .write_off_stock(ctx.pool(), path.into_inner(), claims.sub)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn find_by_barcode(
    ctx: Data<AppContext>,
    path: Path<String>,
) -> Result<HttpResponse, ApiError> {
    let article = ctx
        .article_service()
        .find_by_barcode(&path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(article))
}

async fn create_article(
    ctx: Data<AppContext>,
    upload: ArticleUpload<ArticleCreateRequest>, // Magija: Actix sve završava u pozadini!
) -> Result<HttpResponse, ApiError> {
    let new_id = ctx
        .article_service()
        .create_article(ctx.pool(), upload.dto, upload.slika)
        .await?;
    Ok(HttpResponse::Created().json(new_id))
}

async fn update_article(
    ctx: Data<AppContext>,
    path: Path<u64>,
    upload: ArticleUpload<ArticleUpdateRequest>, // Isti ekstraktor, drugi DTO model!
) -> Result<HttpResponse, ApiError> {
    ctx.article_service()
        .update_article(ctx.pool(), path.into_inner(), upload.dto, upload.slika)
        .await?;
    Ok(HttpResponse::Ok().json("Artikal uspešno ažuriran"))
}

async fn delete_article(ctx: Data<AppContext>, path: Path<u64>) -> Result<HttpResponse, ApiError> {
    ctx.article_service()
        .delete_article(ctx.pool(), path.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

async fn get_all_category_names(ctx: Data<AppContext>) -> Result<HttpResponse, ApiError> {
    let categories = ctx
        .article_service()
        .get_all_category_names(ctx.pool())
        .await?;
    Ok(HttpResponse::Ok().json(categories))
}
