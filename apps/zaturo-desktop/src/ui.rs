//! Safe typed presentation facade over Slint's official interpreter.
//! Embedded Slint source is checked at build time; no generated FFI enters this crate.
use crate::{
    api::{FeedItem, FeedPage},
    timeline::{FeedTimeline, MAX_ITEMS, MAX_OFFSET},
};
use anyhow::Result;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use slint_interpreter::{Compiler, ComponentInstance, Struct, Value};
use std::sync::{Arc, Mutex};
#[derive(Clone, Default)]
pub struct FeedRow {
    pub id: SharedString,
    pub time: SharedString,
    pub label: SharedString,
    pub text: SharedString,
    pub role: SharedString,
    pub task: SharedString,
    pub artifact: SharedString,
}
#[derive(Clone, Default)]
pub struct WorkerRow {
    pub id: SharedString,
    pub detail: SharedString,
}
pub struct AppWindow {
    instance: ComponentInstance,
    timeline: Arc<Mutex<FeedTimeline>>,
    filter_key: Arc<Mutex<Option<String>>>,
}
#[derive(Clone)]
pub struct UiWeak {
    component: slint::Weak<ComponentInstance>,
    timeline: Arc<Mutex<FeedTimeline>>,
    filter_key: Arc<Mutex<Option<String>>>,
}
impl UiWeak {
    pub fn upgrade(&self) -> Option<AppWindow> {
        self.component.upgrade().map(|instance| AppWindow {
            instance,
            timeline: self.timeline.clone(),
            filter_key: self.filter_key.clone(),
        })
    }
    pub fn upgrade_in_event_loop(
        &self,
        callback: impl FnOnce(AppWindow) + Send + 'static,
    ) -> Result<(), slint::EventLoopError> {
        let timeline = self.timeline.clone();
        let filter_key = self.filter_key.clone();
        self.component.upgrade_in_event_loop(move |instance| {
            callback(AppWindow {
                instance,
                timeline,
                filter_key,
            })
        })
    }
}
macro_rules! strings {($(($set:ident,$get:ident,$name:literal)),*$(,)?)=>{$(pub fn $set(&self,value:SharedString){self.instance.set_property($name,Value::String(value)).expect("embedded UI property");}pub fn $get(&self)->SharedString {match self.instance.get_property($name).expect("embedded UI property"){Value::String(s)=>s,_=>panic!("embedded UI type mismatch")}})*};}
macro_rules! booleans {($(($set:ident,$get:ident,$name:literal)),*$(,)?)=>{$(pub fn $set(&self,value:bool){self.instance.set_property($name,Value::Bool(value)).expect("embedded UI property");}pub fn $get(&self)->bool {match self.instance.get_property($name).expect("embedded UI property"){Value::Bool(value)=>value,_=>panic!("embedded UI type mismatch")}})*};}
macro_rules! callbacks {($(($method:ident,$name:literal)),*$(,)?)=>{$(pub fn $method(&self,callback:impl Fn()+'static){self.instance.set_callback($name,move |_|{callback();Value::Void}).expect("embedded UI callback");})*};}
impl AppWindow {
    pub fn new() -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let mut compiler = Compiler::default();
        compiler.set_style("fluent-dark".into());
        let result = runtime.block_on(compiler.build_from_source(
            include_str!("../ui/app-window.slint").into(),
            "embedded-app.slint".into(),
        ));
        let definition = result
            .component("AppWindow")
            .ok_or_else(|| anyhow::anyhow!("embedded UI failed validation"))?;
        let instance = definition.create()?;
        let scroll_target = instance.as_weak();
        instance.set_callback("settle-scroll", move |_| {
            if let Some(instance) = scroll_target.upgrade() {
                let revision = property_number(&instance, "scroll-revision") as i32;
                settle_scroll(scroll_target.clone(), revision, 0, None);
            }
            Value::Void
        })?;
        instance.set_property(
            "brand-icon",
            Value::Image(slint::Image::load_from_svg_data(include_bytes!(
                "../../../assets/brand/zaturobot-icon.svg"
            ))?),
        )?;
        let icons: [(&str, &[u8]); 16] = [
            (
                "icon-chat",
                include_bytes!("../assets/material-symbols-rounded/ic_chat.svg"),
            ),
            (
                "icon-services",
                include_bytes!("../assets/material-symbols-rounded/ic_services.svg"),
            ),
            (
                "icon-sync",
                include_bytes!("../assets/material-symbols-rounded/ic_sync.svg"),
            ),
            (
                "icon-settings",
                include_bytes!("../assets/material-symbols-rounded/ic_settings.svg"),
            ),
            (
                "icon-send",
                include_bytes!("../assets/material-symbols-rounded/ic_send.svg"),
            ),
            (
                "icon-filter",
                include_bytes!("../assets/material-symbols-rounded/ic_filter.svg"),
            ),
            (
                "icon-search",
                include_bytes!("../assets/material-symbols-rounded/ic_search.svg"),
            ),
            (
                "icon-calendar",
                include_bytes!("../assets/material-symbols-rounded/ic_calendar.svg"),
            ),
            (
                "icon-terminal",
                include_bytes!("../assets/material-symbols-rounded/ic_terminal.svg"),
            ),
            (
                "icon-youtube",
                include_bytes!("../assets/material-symbols-rounded/ic_youtube.svg"),
            ),
            (
                "icon-phone",
                include_bytes!("../assets/material-symbols-rounded/ic_phone.svg"),
            ),
            (
                "icon-shield",
                include_bytes!("../assets/material-symbols-rounded/ic_shield.svg"),
            ),
            (
                "icon-folder",
                include_bytes!("../assets/material-symbols-rounded/ic_folder.svg"),
            ),
            (
                "icon-check",
                include_bytes!("../assets/material-symbols-rounded/ic_check.svg"),
            ),
            (
                "icon-close",
                include_bytes!("../assets/material-symbols-rounded/ic_close.svg"),
            ),
            (
                "icon-key",
                include_bytes!("../assets/material-symbols-rounded/ic_key.svg"),
            ),
        ];
        for (name, bytes) in icons {
            instance.set_property(name, Value::Image(slint::Image::load_from_svg_data(bytes)?))?;
        }
        Ok(Self {
            instance,
            timeline: Arc::new(Mutex::new(FeedTimeline::default())),
            filter_key: Arc::new(Mutex::new(None)),
        })
    }
    pub fn as_weak(&self) -> UiWeak {
        UiWeak {
            component: self.instance.as_weak(),
            timeline: self.timeline.clone(),
            filter_key: self.filter_key.clone(),
        }
    }
    pub fn run(&self) -> Result<(), slint::PlatformError> {
        self.instance.run()
    }
    pub fn window(&self) -> &slint::Window {
        self.instance.window()
    }
    strings!(
        (set_private_device, get_private_device, "private-device"),
        (set_private_watch, get_private_watch, "private-watch"),
        (set_private_status, get_private_status, "private-status"),
        (set_page, get_page, "page"),
        (set_selected_day, get_selected_day, "selected-day"),
        (set_search_query, get_search_query, "search-query"),
        (set_composer, get_composer, "composer"),
        (set_worker_target, get_worker_target, "worker-target"),
        (set_connection_url, get_connection_url, "connection-url"),
        (
            set_connection_token,
            get_connection_token,
            "connection-token"
        ),
        (
            set_connection_state,
            get_connection_state,
            "connection-state"
        ),
        (set_status_message, get_status_message, "status-message"),
        (
            set_services_summary,
            get_services_summary,
            "services-summary"
        )
    );
    booleans!(
        (
            set_private_confirmed,
            get_private_confirmed,
            "private-confirmed"
        ),
        (set_private_running, get_private_running, "private-running"),
        (
            set_private_image_visible,
            get_private_image_visible,
            "private-image-visible"
        ),
        (set_online, get_online, "online"),
        (set_busy, get_busy, "busy"),
        (set_sending, get_sending, "sending"),
        (set_has_more, get_has_more, "has-more"),
        (set_preview_open, get_preview_open, "preview-open")
    );
    callbacks!(
        (on_start_private, "start-private"),
        (on_close_private, "close-private"),
        (on_refresh, "refresh"),
        (on_filters_changed, "filters-changed"),
        (on_send_message, "send-message"),
        (on_diagnostic, "diagnostic"),
        (on_save_connection, "save-connection"),
        (on_load_more, "load-more"),
        (on_hub_opened, "hub-opened")
    );
    pub fn set_kind_index(&self, value: i32) {
        self.instance
            .set_property("kind-index", Value::Number(value.into()))
            .expect("embedded kind index");
    }
    pub fn get_following_tail(&self) -> bool {
        matches!(
            self.instance.get_property("following-tail"),
            Ok(Value::Bool(true))
        )
    }
    pub fn get_kind_index(&self) -> i32 {
        match self
            .instance
            .get_property("kind-index")
            .expect("embedded UI property")
        {
            Value::Number(n) => n as i32,
            _ => 0,
        }
    }
    pub fn on_open_artifact(&self, callback: impl Fn(SharedString) + 'static) {
        self.instance
            .set_callback("open-artifact", move |values| {
                if let Some(Value::String(id)) = values.first() {
                    callback(id.clone());
                }
                Value::Void
            })
            .expect("embedded UI callback");
    }
    pub fn on_service_action(&self, callback: impl Fn(SharedString) + 'static) {
        self.instance
            .set_callback("service-action", move |values| {
                if let Some(Value::String(id)) = values.first() {
                    callback(id.clone());
                }
                Value::Void
            })
            .expect("embedded UI callback");
    }
    /// Return chronological rows only when visible state changed; preserve raw server paging.
    pub fn merge_page(&self, page: FeedPage, append: bool, reset: bool) -> Option<Vec<FeedItem>> {
        let mut timeline = self.timeline.lock().expect("UI timeline lock");
        let replaced = reset && (!timeline.items().is_empty() || timeline.has_more());
        if reset {
            timeline.clear();
        }
        let changed =
            timeline.merge_counted(page.items, page.received_count, page.has_more, append)
                || replaced;
        self.set_has_more(
            timeline.has_more()
                && timeline.items().len() < MAX_ITEMS
                && timeline.next_offset() < MAX_OFFSET,
        );
        changed.then(|| timeline.items().iter().rev().cloned().collect())
    }
    pub fn feed_filter_matches(&self, key: &str) -> bool {
        self.filter_key.lock().expect("UI filter lock").as_deref() == Some(key)
    }
    pub fn select_feed_filter(&self, key: String) -> bool {
        let mut current = self.filter_key.lock().expect("UI filter lock");
        let changed = current.as_ref() != Some(&key);
        *current = Some(key);
        changed
    }
    pub fn clear_feed(&self) {
        *self.filter_key.lock().expect("UI filter lock") = None;
        self.timeline.lock().expect("UI timeline lock").clear();
        self.set_events(ModelRc::new(VecModel::from(Vec::<FeedRow>::new())));
        self.set_has_more(false);
    }
    pub fn feed_next_offset(&self) -> u32 {
        self.timeline
            .lock()
            .expect("UI timeline lock")
            .next_offset()
    }
    pub fn get_at_bottom(&self) -> bool {
        matches!(
            self.instance.get_property("at-bottom"),
            Ok(Value::Bool(true))
        )
    }
    pub fn prepare_scroll(&self, mode: i32) {
        self.instance
            .invoke("prepare-scroll", &[Value::Number(mode.into())])
            .expect("embedded scroll function");
    }
    pub fn scroll_to_bottom(&self) {
        self.instance
            .invoke("scroll-to-bottom", &[])
            .expect("embedded scroll function");
    }
    pub fn open_hub(&self) {
        self.instance
            .invoke("open-hub", &[])
            .expect("embedded hub function");
    }
    fn number(&self, name: &str) -> f32 {
        match self
            .instance
            .get_property(name)
            .expect("embedded UI property")
        {
            Value::Number(value) => value as f32,
            _ => panic!("embedded UI type mismatch"),
        }
    }
    pub fn chat_scroll_y(&self) -> f32 {
        self.number("chat-scroll-y")
    }
    pub fn chat_total_height(&self) -> f32 {
        self.number("chat-total-height")
    }
    pub fn chat_visible_height(&self) -> f32 {
        self.number("chat-visible-height")
    }
    /// Used by the isolated UI smoke to model a reader selecting an earlier viewport.
    pub fn set_chat_scroll_y(&self, value: f32) {
        self.instance
            .set_property("chat-scroll-y", Value::Number(value.into()))
            .expect("embedded scroll property");
    }
    pub fn invoke_refresh(&self) {
        let _ = self.instance.invoke("refresh", &[]);
    }
    pub fn set_private_image(&self, image: slint::Image) {
        self.instance
            .set_property("private-image", Value::Image(image))
            .expect("embedded UI property");
    }
    pub fn set_preview_image(&self, image: slint::Image) {
        self.instance
            .set_property("preview-image", Value::Image(image))
            .expect("embedded UI property");
    }
    pub fn get_events(&self) -> ModelRc<FeedRow> {
        let rows = match self
            .instance
            .get_property("events")
            .expect("embedded UI property")
        {
            Value::Model(model) => model
                .iter()
                .filter_map(|row| match row {
                    Value::Struct(row) => Some(FeedRow {
                        id: field(&row, "id"),
                        time: field(&row, "time"),
                        label: field(&row, "label"),
                        text: field(&row, "text"),
                        role: field(&row, "role"),
                        task: field(&row, "task"),
                        artifact: field(&row, "artifact"),
                    }),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        ModelRc::new(VecModel::from(rows))
    }
    pub fn set_events(&self, rows: ModelRc<FeedRow>) {
        let values = rows
            .iter()
            .map(|r| {
                object(vec![
                    ("id", r.id),
                    ("time", r.time),
                    ("label", r.label),
                    ("text", r.text),
                    ("role", r.role),
                    ("task", r.task),
                    ("artifact", r.artifact),
                ])
            })
            .collect::<Vec<_>>();
        self.instance
            .set_property("events", Value::Model(ModelRc::new(VecModel::from(values))))
            .expect("embedded UI property");
    }
    pub fn set_workers(&self, rows: ModelRc<WorkerRow>) {
        let values = rows
            .iter()
            .map(|r| object(vec![("id", r.id), ("detail", r.detail)]))
            .collect::<Vec<_>>();
        self.instance
            .set_property(
                "workers",
                Value::Model(ModelRc::new(VecModel::from(values))),
            )
            .expect("embedded UI property");
    }
}
fn object(fields: Vec<(&str, SharedString)>) -> Value {
    Value::Struct(
        fields
            .into_iter()
            .map(|(name, value)| (name.into(), Value::String(value)))
            .collect(),
    )
}
fn field(object: &Struct, key: &str) -> SharedString {
    match object.get_field(key) {
        Some(Value::String(value)) => value.clone(),
        _ => SharedString::default(),
    }
}

/// Apply viewport intent after geometry settles, without keeping the component alive.
/// A new intent or real user scroll cancels any older deferred work.
fn settle_scroll(
    target: slint::Weak<ComponentInstance>,
    revision: i32,
    attempt: u8,
    previous: Option<(f64, f64)>,
) {
    slint::Timer::single_shot(std::time::Duration::from_millis(16), move || {
        let Some(instance) = target.upgrade() else {
            return;
        };
        if property_number(&instance, "scroll-revision") as i32 != revision {
            return;
        }
        let mode = property_number(&instance, "scroll-mode") as i32;
        if mode == 0 {
            return;
        }
        let page = instance.get_property("page").expect("embedded page");
        if !matches!(page, Value::String(ref value) if value == "hub") {
            instance
                .set_property("scroll-mode", Value::Number(0.0))
                .expect("embedded scroll state");
            return;
        }
        let total = property_number(&instance, "chat-total-height");
        let visible = property_number(&instance, "chat-visible-height");
        if visible > 0.0 {
            let minimum = (visible - total).min(0.0);
            let wanted = if mode == 1 {
                minimum
            } else {
                let base = property_number(&instance, "scroll-base-y");
                let change = if mode == 2 {
                    total - property_number(&instance, "scroll-base-height")
                } else {
                    0.0
                };
                (base - change).clamp(minimum, 0.0)
            };
            instance
                .set_property("chat-scroll-y", Value::Number(wanted))
                .expect("embedded scroll position");
            let stable = previous.is_some_and(|(old_total, old_visible)| {
                (old_total - total).abs() < 0.5 && (old_visible - visible).abs() < 0.5
            });
            if stable && (property_number(&instance, "chat-scroll-y") - wanted).abs() < 0.5 {
                instance
                    .set_property("scroll-mode", Value::Number(0.0))
                    .expect("embedded scroll state");
                return;
            }
        }
        if attempt < 31 {
            settle_scroll(target, revision, attempt + 1, Some((total, visible)));
        } else {
            // A later metrics update still retargets a tail-following viewport.
            instance
                .set_property("scroll-mode", Value::Number(0.0))
                .expect("embedded scroll state");
        }
    });
}
fn property_number(instance: &ComponentInstance, name: &str) -> f64 {
    match instance
        .get_property(name)
        .expect("embedded viewport property")
    {
        Value::Number(value) => value,
        _ => panic!("embedded viewport type mismatch"),
    }
}
