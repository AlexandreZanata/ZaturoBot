//! Safe typed presentation facade over Slint's official interpreter.
//! Embedded Slint source is checked at build time; no generated FFI enters this crate.
use anyhow::Result;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use slint_interpreter::{Compiler, ComponentInstance, Struct, Value};
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
}
#[derive(Clone)]
pub struct UiWeak(slint::Weak<ComponentInstance>);
impl UiWeak {
    pub fn upgrade(&self) -> Option<AppWindow> {
        self.0.upgrade().map(|instance| AppWindow { instance })
    }
    pub fn upgrade_in_event_loop(
        &self,
        callback: impl FnOnce(AppWindow) + Send + 'static,
    ) -> Result<(), slint::EventLoopError> {
        self.0
            .upgrade_in_event_loop(move |instance| callback(AppWindow { instance }))
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
        instance.set_property(
            "brand-icon",
            Value::Image(slint::Image::load_from_svg_data(include_bytes!(
                "../../../assets/brand/zaturobot-icon.svg"
            ))?),
        )?;
        Ok(Self { instance })
    }
    pub fn as_weak(&self) -> UiWeak {
        UiWeak(self.instance.as_weak())
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
        (on_load_more, "load-more")
    );
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
