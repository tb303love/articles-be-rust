use crate::context::AppContext;
use crate::dto::ArticleCreateRequest;
use actix_web::web::{self, Data, Json, Path, Query};
use actix_web::HttpResponse;
use serde::Deserialize;

// --- STRUKTURE ZA AUTOMATSKU VALIDACIJU QUERY PARAMETARA ---

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

// --- CENTRALNA REGISTRACIJA (SAVRŠENA ENKAPSULACIJA) ---

pub fn rute(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/articles")
            // Rute su čiste i direktno mapirane na funkcije ispod
            .route("", web::get().to(get_all_articles))
            .route("", web::post().to(create_article))
            .route("/check-name", web::get().to(check_name))
            .route("/check-name-exclude", web::get().to(check_name_exclude))
            .route(
                "/check-name-exclude-specific",
                web::get().to(check_name_exclude_specific),
            )
            .route(
                "/affected-bundles/{stock_id}",
                web::get().to(find_affected_bundles),
            )
            .route("/barcode/{code}", web::get().to(find_by_barcode))
            .route("/{id}", web::put().to(update_article))
            .route("/{id}", web::delete().to(delete_article)),
    );
}

// --- HANDLERI (ČISTE FUNKCIJE SA EKSPLICITNIM IZVLAČENJEM) ---

async fn get_all_articles(ctx: Data<AppContext>, query: Query<SearchQuery>) -> HttpResponse {
    let service = ctx.article_service();
    match service.get_all_articles(query.into_inner().query).await {
        Ok(articles) => HttpResponse::Ok().json(articles),
        Err(e) => {
            log::error!("Greška pri čitanju artikala iz baze: {:?}", e);
            HttpResponse::InternalServerError().body("Greška na serveru")
        }
    }
}

async fn check_name(ctx: Data<AppContext>, query: Query<NameCheckQuery>) -> HttpResponse {
    let service = ctx.article_service();
    match service.check_name_availability(&query.name).await {
        Ok(is_available) => HttpResponse::Ok().json(is_available),
        Err(e) => {
            log::error!("Greška pri proveri imena artikla: {:?}", e);
            HttpResponse::InternalServerError().body("Greška na serveru")
        }
    }
}

async fn check_name_exclude(ctx: Data<AppContext>, query: Query<NameExcludeQuery>) -> HttpResponse {
    let service = ctx.article_service();
    let q = query.into_inner();
    match service
        .find_article_by_article_name_exclude_specific(&q.name, q.id)
        .await
    {
        Ok(is_available) => HttpResponse::Ok().json(is_available),
        Err(e) => {
            log::error!("Greška pri proveri izuzetog imena artikla: {:?}", e);
            HttpResponse::InternalServerError().body("Greška na serveru")
        }
    }
}

async fn check_name_exclude_specific(
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
            log::error!("Greška pri proveri specifičnog izuzetog imena: {:?}", e);
            HttpResponse::InternalServerError().body("Greška na serveru")
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
            HttpResponse::InternalServerError().body("Greška na serveru")
        }
    }
}

async fn find_by_barcode(ctx: Data<AppContext>, path: Path<String>) -> HttpResponse {
    let service = ctx.article_service();
    let code = path.into_inner();
    match service.find_by_barcode(&code).await {
        Ok(article) => HttpResponse::Ok().json(article),
        Err(e) => {
            log::error!(
                "Greška pri pronalaženju artikla preko barkoda {}: {:?}",
                code,
                e
            );
            HttpResponse::InternalServerError().body("Greška na serveru")
        }
    }
}

async fn create_article(ctx: Data<AppContext>, body: Json<ArticleCreateRequest>) -> HttpResponse {
    let service = ctx.article_service();

    // Validacija imena
    match service.check_name_availability(&body.name).await {
        Ok(is_available) => {
            if !is_available {
                return HttpResponse::BadRequest().body("Artikal sa tim imenom već postoji");
            }
        }
        Err(err) => {
            log::error!("Greška pri validaciji imena za novi artikal: {:?}", err);
            return HttpResponse::InternalServerError().body("Greška na serveru");
        }
    }

    // Izvršavanje unosa (koristimo pool direktno iz ctx)
    match service.create_article(ctx.pool(), body.into_inner()).await {
        Ok(new_id) => HttpResponse::Created().json(new_id),
        Err(err) => {
            log::error!("Greška pri kreiranju artikla u bazi: {:?}", err);
            HttpResponse::InternalServerError().body("Greška na serveru prilikom upisa artikla")
        }
    }
}

async fn update_article(
    ctx: Data<AppContext>,
    path: Path<u64>,
    body: Json<ArticleCreateRequest>,
) -> HttpResponse {
    let service = ctx.article_service();
    let article_id = path.into_inner();

    match service
        .update_article(ctx.pool(), article_id, body.into_inner())
        .await
    {
        Ok(_) => HttpResponse::Ok().body("Artikal uspešno ažuriran"),
        Err(err) => match err {
            sqlx::Error::RowNotFound => HttpResponse::NotFound().body("Artikal nije pronađen"),
            _ => {
                log::error!("Greška pri ažuriranju artikla: {:?}", err);
                HttpResponse::InternalServerError().body("Greška na serveru")
            }
        },
    }
}

async fn delete_article(ctx: Data<AppContext>, path: Path<u64>) -> HttpResponse {
    let service = ctx.article_service();
    let article_id = path.into_inner();

    match service.delete_article(ctx.pool(), article_id).await {
        Ok(_) => HttpResponse::Ok().body("Artikal uspešno obrisan"),
        Err(err) => match err {
            sqlx::Error::RowNotFound => HttpResponse::NotFound().body("Artikal nije pronađen"),
            _ => {
                log::error!("Greška pri brisanju artikla: {:?}", err);
                HttpResponse::InternalServerError().body("Greška na serveru")
            }
        },
    }
}
