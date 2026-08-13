// Required for macro-internal `app_handle()` calls
use tauri::Manager;
// use tauri_nspanel::objc2::rc::Retained;
use tauri_nspanel::tauri_panel;
use tauri_nspanel::TrackingAreaOptions;

tauri_panel! {
    panel!(FloatSearchPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
        with: {
            tracking_area: {
                options: TrackingAreaOptions::new()
                    .active_always()
                    .mouse_entered_and_exited()
                    .mouse_moved(),
                auto_resize: true
            }
        }
    })

    panel!(NotificationPanel {
        config: {
            can_become_key_window: false,
            is_floating_panel: true
        }
    })

    panel_event!(PanelEventHandler {
        window_did_move(notification: &NSNotification) -> (),
        window_did_resize(notification: &NSNotification) -> (),
        window_did_become_key(notification: &NSNotification) -> (),
        window_did_resign_key(notification: &NSNotification) -> ()
    })
}

pub struct PublicPanelEventHandler {
    inner: Retained<PanelEventHandler>,
}

impl PublicPanelEventHandler {
    pub fn new() -> Self {
        Self {
            inner: PanelEventHandler::new(),
        }
    }

    pub fn window_did_move<F>(&self, handler: F)
    where
        F: Fn(&NSNotification) + Send + Sync + 'static,
    {
        self.inner.window_did_move(handler);
    }

    pub fn window_did_resize<F>(&self, handler: F)
    where
        F: Fn(&NSNotification) + Send + Sync + 'static,
    {
        self.inner.window_did_resize(handler);
    }

    pub fn window_did_become_key<F>(&self, handler: F)
    where
        F: Fn(&NSNotification) + Send + Sync + 'static,
    {
        self.inner.window_did_become_key(handler);
    }

    pub fn window_did_resign_key<F>(&self, handler: F)
    where
        F: Fn(&NSNotification) + Send + Sync + 'static,
    {
        self.inner.window_did_resign_key(handler);
    }

    // Inside implementation block for PublicPanelEventHandler in helper/src/panels.rs
    
    /// Provides an open signature converting internal state back to a visible NSWindowDelegate ProtocolObject.
    pub fn as_protocol_delegate(&self) -> &ProtocolObject<dyn NSWindowDelegate> {
        // ✨ Fix: Use the turbofish syntax ::<PanelEventHandler> to guide the compiler's type resolution engine
        ProtocolObject::from_ref::<PanelEventHandler>(self.inner.as_ref())
    }

}
