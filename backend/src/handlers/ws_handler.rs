use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::context::AppContext;
use crate::ws::hub::WsHub;
// Pretpostavka da hub živi u AppContext-u
use crate::ws::protocol::{KlijentPoruka, ServerPoruka};
use serde_json;

#[get("/ws-barcode")]
pub async fn ws_barcode_endpoint(
    req: HttpRequest,
    stream: web::Payload,
    ctx: web::Data<AppContext>,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    // Generišemo jedinstveni ID za ovu konekciju
    let klijent_id = Uuid::new_v4();
    let hub = ctx.ws_hub();

    // Pokrećemo asinhroni zadatak za ovog klijenta
    actix_web::rt::spawn(pokreni_klijentsku_sesiju(
        klijent_id, session, msg_stream, hub,
    ));

    Ok(res)
}

async fn pokreni_klijentsku_sesiju(
    id: Uuid,
    mut session: actix_ws::Session,
    mut msg_stream: actix_ws::MessageStream,
    hub: Arc<WsHub>,
) {
    // Kreiramo kanal preko kojeg će Hub slati poruke OVOM klijentu
    let (tx, mut rx) = mpsc::channel::<ServerPoruka>(32);

    // Registrujemo klijenta u globalni Hub
    hub.registruj(id, tx).await;

    loop {
        tokio::select! {
            // 1. Stigla poruka od servera (iz Hub-a) -> Šaljemo je klijentu na mrežu
            Some(serverska_poruka) = rx.recv() => {
                if let Ok(json) = serde_json::to_string(&serverska_poruka) {
                    if session.text(json).await.is_err() { break; }
                }
            }

            // 2. Klijent je poslao nešto preko WebSocket-a -> Obrađujemo
            Some(Ok(msg)) = msg_stream.next() => {
                match msg {
                    Message::Text(text) => {
                        // Rust-ičan match: Direktno parsujemo u naš enum
                        if let Ok(klijent_poruka) = serde_json::from_str::<KlijentPoruka>(&text) {
                            izvrsi_logiku(id, klijent_poruka, &hub).await;
                        } else {
                            let greska = ServerPoruka::Greska { poruka: "Format poruke nije podržan".to_string() };
                            let _ = session.text(serde_json::to_string(&greska).unwrap()).await;
                        }
                    }
                    Message::Close(reason) => {
                        let _ = session.close(reason).await;
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // Kada se petlja prekine (klijent se diskonektovao), brišemo ga iz Hub-a
    hub.odjavi(id).await;
}

/// Čista Rust biznis logika zasnovana na Pattern Matching-u
async fn izvrsi_logiku(klijent_id: Uuid, poruka: KlijentPoruka, hub: &WsHub) {
    match poruka {
        KlijentPoruka::SkeniranBarkod { kod } => {
            log::info!("Klijent {} skenirao kod: {}", klijent_id, kod);

            // Simulacija obrade i slanje odgovora SVIMA (Broadcast)
            let odgovor = ServerPoruka::BarkodProcesiran {
                kod,
                status: "USPESNO_EVIDENTIRAN".to_string(),
            };
            hub.posalji_svima(odgovor).await;
        }
        KlijentPoruka::PridruziSeGrupi { ime_grupe } => {
            // Ovde može ići logika za sobe/grupe ako zatreba
            hub.posalji_klijentu(
                klijent_id,
                ServerPoruka::Obavestenje {
                    tekst: format!("Ušao si u grupu {}", ime_grupe),
                },
            )
            .await;
        }
        KlijentPoruka::NapustiGrupu { ime_grupe } => {
            log::info!("Klijent {} napustio grupu {}", klijent_id, ime_grupe);
        }
    }
}
