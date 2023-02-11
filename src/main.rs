use log::info;
use relm4::gtk;
use relm4::gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, PopoverExt, WidgetExt};
use relm4::{send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};
use std::sync::{atomic, atomic::AtomicBool, Arc};
use std::time::Duration;

const SLEEP_STEP: Duration = Duration::from_millis(250);

const ICON_START: &str = "media-playback-start-symbolic";
const ICON_PAUSE: &str = "media-playback-pause-symbolic";
const ICON_SKIP: &str = "media-skip-forward-symbolic"; // maybe go-jump-symbolic
const ICON_RENEW: &str = "media-skip-backward-symbolic";
const ICON_RESTART: &str = "object-rotate-left-symbolic";
const ICON_CONFIG: &str = "preferences-system-symbolic"; // maybe applications-system-symbolic

fn main() {
    simple_logger::init_with_env().unwrap();
    let model = AppModel::default();
    let app = RelmApp::new(model);
    app.run();
}

#[relm4_macros::widget]
impl Widgets<AppModel, ()> for AppWidgets {
    view! {
        gtk::ApplicationWindow {
            set_title: Some("Pomo Oxide"),
            set_default_width: 350,
            set_icon_name: Some("pomodoro-indicator"),
            set_resizable: false,

            set_titlebar = Some(&gtk::HeaderBar) {
                pack_start = &gtk::MenuButton {
                    set_icon_name: ICON_CONFIG,

                    set_popover = Some(&gtk::Popover) {
                        set_child = Some(&gtk::Box) {
                            set_orientation: gtk::Orientation::Vertical,

                            append = &gtk::SpinButton {
                                set_tooltip_text: Some("Pomodoro Duration"),
                                set_range: args!(1.0, 180.0),
                                set_value: (model.config.pomodoro_time.as_secs() / 60) as f64,
                                set_increments: args!(1.0, 5.0),

                                connect_value_changed(sender) => move |btn|
                                    send!(sender, AppMsg::ChangeConfig({
                                        let value = btn.value();
                                        Box::new(move |mut config|
                                            config.pomodoro_time = Duration::from_secs(value as u64 * 60)
                                        )
                                })),
                            },

                            append = &gtk::SpinButton {
                                set_tooltip_text: Some("Break Duration"),
                                set_range: args!(1.0, 180.0),
                                set_value: (model.config.break_time.as_secs() / 60) as f64,
                                set_increments: args!(1.0, 5.0),

                                connect_value_changed(sender) => move |btn|
                                    send!(sender, AppMsg::ChangeConfig({
                                        let value = btn.value();
                                        Box::new(move |mut config|
                                            config.break_time = Duration::from_secs(value as u64 * 60)
                                        )
                                })),
                            },

                            append = &gtk::SpinButton {
                                set_tooltip_text: Some("Rest Duration"),
                                set_range: args!(1.0, 180.0),
                                set_value: (model.config.rest_time.as_secs() / 60) as f64,
                                set_increments: args!(1.0, 5.0),

                                connect_value_changed(sender) => move |btn|
                                    send!(sender, AppMsg::ChangeConfig({
                                        let value = btn.value();
                                        Box::new(move |mut config|
                                            config.rest_time = Duration::from_secs(value as u64 * 60)
                                        )
                                })),
                            },

                            append = &gtk::SpinButton {
                                set_tooltip_text: Some("Rest Count - how many pomodoros until the break will be a rest."),
                                set_range: args!(1.0, 20.0),
                                set_value: model.config.rest_count as f64,
                                set_increments: args!(1.0, 2.0),

                                connect_value_changed(sender) => move |btn|
                                    send!(sender, AppMsg::ChangeConfig({
                                        let value = btn.value();
                                        Box::new(move |mut config| config.rest_count = value as u8)
                                })),
                            },
                        }
                    }
                },
            },

            set_child = Some(&gtk::Box) {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 10,

                append = &gtk::Button {
                    set_icon_name: watch!{ if model.running { ICON_PAUSE } else { ICON_START } },
                    set_tooltip_text: watch!{ Some(if model.running { "Pause" } else { "Start" }) },
                    set_height_request: 50,
                    connect_clicked(sender) => move |_| { send!(sender, AppMsg::Toggle(None)) },
                },

                append = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 10,
                    set_height_request: 40,
                    set_homogeneous: true,

                    append = &gtk::Button {
                        set_icon_name: ICON_SKIP,
                        set_tooltip_text: Some("Skip"),
                        connect_clicked(sender) => move |_| { send!(sender, AppMsg::Skip) },
                    },

                    append = &gtk::Button {
                        set_icon_name: ICON_RENEW,
                        set_tooltip_text: watch!{ Some(&format!("Renew {}", model.state)) },
                        connect_clicked(sender) => move |_| { send!(sender, AppMsg::Renew) },
                    },

                    append = &gtk::Button {
                        set_icon_name: ICON_RESTART,
                        set_tooltip_text: Some("Restart"),
                        connect_clicked(sender) => move |_| { send!(sender, AppMsg::Restart) },
                    },
                },

                append = &gtk::Label {
                    set_halign: gtk::Align::Center,
                    set_markup: watch!{ &model.state.as_markup(&model.config) },
                },

                append = &gtk::Label {
                    set_halign: gtk::Align::Center,
                    set_markup: watch!{ &min_as_markup(min_format(&model.timer)) },
                    set_opacity: watch!{ if model.running { 1.0 } else { 0.7 } },
                },
            }
        }
    }
}

#[derive(Debug)]
struct AppModel {
    running: bool,
    timer: Duration,
    state: State,
    rest_counter: usize,
    pomodoro_count: usize,
    config: Config,
    /// Stores if a running thread has permission to send AppMsg::Step
    step_permission: Option<Arc<AtomicBool>>,
}

impl Default for AppModel {
    fn default() -> Self {
        let state = State::default();
        let config = Config::default();
        Self {
            running: false,
            timer: state.duration(&config),
            state,
            rest_counter: 0,
            pomodoro_count: 0,
            config,
            step_permission: None,
        }
    }
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Pomodoro,
    Break,
    Rest,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl State {
    fn duration(&self, config: &Config) -> Duration {
        match self {
            State::Pomodoro => config.pomodoro_time,
            State::Break => config.break_time,
            State::Rest => config.rest_time,
        }
    }

    fn as_markup(&self, _config: &Config) -> String {
        // TODO: set colors from Config
        let color = match self {
            State::Pomodoro => "#FFA3CC",
            State::Break => "#FAFFA3",
            State::Rest => "#A3FFD6",
        };
        format!(
            r#"<span font="Sans Bold 28" color="{}">{}</span>"#,
            color, self
        )
    }
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = ();
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: Self::Msg, _: &Self::Components, sender: Sender<Self::Msg>) -> bool {
        self.clear_step_permission();

        match msg {
            AppMsg::Step => self.try_next_state(),
            AppMsg::Toggle(toggle) => self.toggle(toggle),
            AppMsg::Skip => self.next_state(),
            AppMsg::Renew => self.restart_state(),
            AppMsg::Restart => self.restart(),
            AppMsg::ChangeConfig(config_changer) => {
                config_changer(&mut self.config);
                // TODO: Save config
                if self.config.rest_count as usize <= self.rest_counter {
                    self.restart();
                } else {
                    self.restart_state();
                }
            }
        }

        if self.running {
            let duration = SLEEP_STEP.min(self.timer);
            self.timer -= duration;

            self.clear_step_permission();
            let perm = Arc::new(AtomicBool::new(true));
            self.step_permission = Some(Arc::clone(&perm));

            std::thread::spawn(move || {
                std::thread::sleep(duration);
                // if we still have permission, send step
                if perm.load(atomic::Ordering::SeqCst) {
                    sender.send(AppMsg::Step).unwrap();
                }
            });
        }

        true
    }
}

impl AppModel {
    fn toggle(&mut self, running: Option<bool>) {
        self.running = running.unwrap_or(!self.running);
    }
    fn clear_step_permission(&mut self) {
        if let Some(perm) = self.step_permission.take() {
            perm.store(false, atomic::Ordering::SeqCst);
        }
    }
    fn try_next_state(&mut self) {
        if self.timer.is_zero() {
            self.next_state()
        }
    }
    fn next_state(&mut self) {
        self.state = match self.state {
            State::Pomodoro => {
                self.pomodoro_count += 1;
                self.rest_counter += 1;

                if self.rest_counter >= self.config.rest_count as usize {
                    self.rest_counter = 0;
                    State::Rest
                } else {
                    State::Break
                }
            }
            State::Break | State::Rest => State::Pomodoro,
        };
        self.restart_state()
    }
    fn restart_state(&mut self) {
        self.timer = self.state.duration(&self.config);
        info!("Starting {:?} - {}", &self.state, min_format(&self.timer));
    }
    fn restart(&mut self) {
        self.state = State::Pomodoro;
        self.rest_counter = 0;
        self.restart_state();
    }
}

#[allow(unused)]
enum AppMsg {
    Step,
    Toggle(Option<bool>),
    Skip,
    Renew,
    Restart,
    ChangeConfig(Box<dyn FnOnce(&mut Config) + Send>),
}

#[derive(Debug, Clone)]
struct Config {
    /// Time for each pomodoro.
    pomodoro_time: Duration,
    /// Time for each break.
    break_time: Duration,
    /// Time for each rest.
    rest_time: Duration,
    /// How many pomodoros until the break will be a rest.
    rest_count: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pomodoro_time: Duration::from_secs(60 * 25),
            break_time: Duration::from_secs(60 * 5),
            rest_time: Duration::from_secs(60 * 20),
            rest_count: 4,
        }
    }
}

fn min_format(dur: &Duration) -> String {
    // simulating div_ceil(); it was unstable
    let millis = dur.as_millis();
    let secs = (millis / 1000) + if (millis % 1000) != 0 { 1 } else { 0 };
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn min_as_markup(s: String) -> String {
    format!("<span font=\"Sans Bold 64\">{}</span>", s)
}
