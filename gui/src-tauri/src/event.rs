use std::sync::{mpsc, mpsc::Sender};
use tauri::AppHandle;

pub trait EventHandler<E: Send + 'static> {
    fn is_terminal(&self, event: &E) -> bool;
    fn handle(&self, event: &E, app_handle: &AppHandle);
}

pub struct EventChannel<E: Send + 'static> {
    tx: Sender<E>,
}

impl<E: Send + 'static> EventChannel<E> {
    pub fn new<H: EventHandler<E> + Send + Sync + 'static>(app_handle: &AppHandle, handler: H) -> Self {
        let (tx, rx) = mpsc::channel::<E>();
        let app_handle = app_handle.clone();

        tauri::async_runtime::spawn(async move {
            for event in rx {
                handler.handle(&event, &app_handle);
                if handler.is_terminal(&event) {
                    break;
                }
            }
        });

        Self { tx }
    }

    pub fn sender(&self) -> &Sender<E> {
        &self.tx
    }
}
