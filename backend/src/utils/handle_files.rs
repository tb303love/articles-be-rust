use actix_files::{Files, NamedFile};
use actix_web::{web, HttpRequest, HttpResponse};
use std::path::PathBuf;

pub fn handle_files(index_putanja: PathBuf, dir_za_radnika: &String) -> Files {
    Files::new("/", &dir_za_radnika)
        .index_file("index.html")
        // Prikaz strukture foldera isključujemo u produkciji radi bezbednosti
        .show_files_listing()
        .default_handler(web::to(move |req: HttpRequest| {
            let putanja_za_handler = index_putanja.clone();
            async move {
                // IZMENJENO: Otvaramo index.html sa dinamičke lokacije
                match NamedFile::open_async(&putanja_za_handler).await {
                    Ok(file) => file.into_response(&req),
                    Err(e) => {
                        log::error!("Kritična greška: Angular index.html nije pronađen na lokaciji {:?}: {:?}", putanja_za_handler, e);
                        HttpResponse::NotFound().body("Frontend nije dostupan.")
                    }
                }
            }
        }))
}
