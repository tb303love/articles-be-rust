// src/ws/hub.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use super::protocol::ServerPoruka;

pub struct WsHub {
    // Čuvamo kanale za svakog spojenog klijenta preko njegovog jedinstvenog ID-ja
    klijenti: Mutex<HashMap<Uuid, mpsc::Sender<ServerPoruka>>>,
}

impl WsHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            klijenti: Mutex::new(HashMap::new()),
        })
    }

    // Registracija novog klijenta pri povezivanju
    pub async fn registruj(&self, id: Uuid, tx: mpsc::Sender<ServerPoruka>) {
        self.klijenti.lock().await.insert(id, tx);
    }

    // Uklanjanje klijenta kada pukne veza
    pub async fn odjavi(&self, id: Uuid) {
        self.klijenti.lock().await.remove(&id);
    }

    // Slanje poruke SVIM spojenim klijentima (Broadcast)
    pub async fn posalji_svima(&self, poruka: ServerPoruka) {
        let klijenti = self.klijenti.lock().await;
        for tx in klijenti.values() {
            let _ = tx.send(poruka.clone()).await;
        }
    }

    // Slanje poruke TAČNO ODREĐENOM klijentu (Direct Message)
    pub async fn posalji_klijentu(&self, id: Uuid, poruka: ServerPoruka) {
        let klijenti = self.klijenti.lock().await;
        if let Some(tx) = klijenti.get(&id) {
            let _ = tx.send(poruka).await;
        }
    }
}
