use std::sync::mpsc;
use tauri::AppHandle;

pub trait EventHandler<E: Send + 'static> {
    fn is_terminal(&self, event: &E) -> bool;
    fn handle(&self, event: &E, app_handle: &AppHandle);
}

pub fn listen<H, E>(rx: mpsc::Receiver<E>, app_handle: &AppHandle, handler: H)
where
    H: EventHandler<E> + Send + 'static,
    E: Send + 'static,
{
    let app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        for event in rx {
            handler.handle(&event, &app_handle);
            if handler.is_terminal(&event) {
                break;
            }
        }
    });
}
