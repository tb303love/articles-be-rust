use std::ffi::OsString;
use std::sync::mpsc;
use std::time::Duration;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::{define_windows_service, service_dispatcher};

const SERVICE_NAME: &str = "OgilenkoServer2";

define_windows_service!(ffi_service_main, win_service_main);

pub fn pokreni_windows_servis() -> Result<(), windows_service::Error> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}

fn win_service_main(_arguments: Vec<OsString>) {
    // UKLONJENO: Promena direktorijuma je uklonjena odavde jer je već rešena u main.rs
    // pre nego što se učita konfiguracija, čime se izbegava konflikt niti.

    if let Err(e) = run_service_loop() {
        log::error!("Greška u radu Windows servisa: {:?}", e);
    }
}

fn run_service_loop() -> Result<(), Box<dyn std::error::Error>> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let (handle_tx, handle_rx) = mpsc::channel();

    let shutdown_tx_for_handler = shutdown_tx.clone();
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                let _ = shutdown_tx_for_handler.send(());
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    // 1. JAVLJAMO WINDOWSU DA SE SERVIS BUDI
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 1,
        wait_hint: Duration::from_secs(15),
        process_id: None,
    })?;

    // 2. POKREĆEMO ACTIX U POZADINSKOJ NITI
    let status_handle_clone = status_handle.clone();
    let shutdown_tx_for_thread = shutdown_tx.clone();

    std::thread::spawn(move || {
        let status_handle = status_handle_clone;
        let shutdown_tx = shutdown_tx_for_thread;

        let sys = actix_web::rt::System::new();
        sys.block_on(async {
            match crate::pokreni_actix_server().await {
                Ok((actix_handle, server)) => {
                    // Prosleđujemo handle glavnoj niti za gašenje
                    let _ = handle_tx.send(actix_handle);

                    // Menjamo status u Running jer je server spreman
                    let _ = status_handle.set_service_status(ServiceStatus {
                        service_type: ServiceType::OWN_PROCESS,
                        current_state: ServiceState::Running,
                        controls_accepted: ServiceControlAccept::STOP,
                        exit_code: ServiceExitCode::Win32(0),
                        checkpoint: 0,
                        wait_hint: Duration::default(),
                        process_id: None,
                    });

                    // Server sada blokira nit i stabilno ostaje u memoriji slušajući zahteve
                    if let Err(e) = server.await {
                        log::error!("Actix Web server je neočekivano stao: {:?}", e);
                    }
                }
                Err(e) => {
                    log::error!("Actix Web server nije uspeo da se inicijalizuje: {:?}", e);
                    let _ = status_handle.set_service_status(ServiceStatus {
                        service_type: ServiceType::OWN_PROCESS,
                        current_state: ServiceState::Stopped,
                        controls_accepted: ServiceControlAccept::empty(),
                        exit_code: ServiceExitCode::Win32(1067),
                        checkpoint: 0,
                        wait_hint: Duration::default(),
                        process_id: None,
                    });
                    let _ = shutdown_tx.send(());
                }
            }
        });
    });

    // Sačekamo Actix handle iz pozadinske niti
    let actix_server_handle = handle_rx.recv().ok();

    if actix_server_handle.is_none() {
        return Ok(());
    }

    // 3. GLAVNA NIT ČEKA SIGNAL ZA GAŠENJE (Korisnik kliknuo "Stop" u Services)
    let _ = shutdown_rx.recv();

    // 4. ELEGANTNO GAŠENJE ACTIX SERVERA PRE NEGO ŠTO REAGUJEMO PREMA SCM-u
    if let Some(handle) = actix_server_handle {
        log::info!("Primljen stop signal. Zaustavljam Actix Web server...");

        // POBOLJŠANO: Koristimo lagani osnovni runtime za slanje stop signala umesto teškog System podsistema
        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            rt.block_on(async {
                handle.stop(true).await; // `true` označava graceful shutdown (sačekaće aktivne zahteve)
            });
        }
    }

    // 5. KORISNIK JE KLIKNUO "STOP" -> GASIMO WINDOWS SERVIS I JAVLJAMO SCM-u
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}
