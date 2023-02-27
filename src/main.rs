use directories::ProjectDirs;
use gtk::prelude::*;
use log::*;
use playback_rs::{Player, Song};
use relm4::prelude::*;
use relm4_components::open_dialog::*;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{atomic, atomic::AtomicBool, Arc};
use std::time::Duration;

const SLEEP_STEP: Duration = Duration::from_millis(250);

const ICON_START: &str = "media-playback-start-symbolic";
const ICON_PAUSE: &str = "media-playback-pause-symbolic";
const ICON_SKIP: &str = "media-skip-forward-symbolic"; // maybe go-jump-symbolic
const ICON_RENEW: &str = "media-skip-backward-symbolic";
const ICON_RESTART: &str = "object-rotate-left-symbolic";
const ICON_CONFIG: &str = "preferences-system-symbolic"; // maybe applications-system-symbolic

const CONFIG_FILE: &str = "config.bin";
const DEFAULT_SOUND: &str = "default.ogg";

fn main() {
    simple_logger::init_with_env().unwrap();

    let dirs = ProjectDirs::from("", "", "pomo-oxide").expect("couldn't get project directories");
    let config_file = dirs.config_dir().join(CONFIG_FILE);
    std::fs::create_dir_all(dirs.config_dir())
        .unwrap_or_else(|e| warn!("Error creating config directory: {e}"));

    let config = {
        (|| -> Result<Config, Box<dyn std::error::Error>> {
            let input = read_to_string(&config_file)?;
            Ok(serde_json::from_str(&input)?)
        })()
        .map_err(|e| warn!("Error loading config: {e}"))
        .unwrap_or_default()
    };

    let model = AppModel::new(config, config_file);
    let app = RelmApp::new("pomo-oxide");
    app.run::<AppModel>(model);
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Input = AppMsg;
    type Output = ();
    type Init = Self;

    fn init(
        model: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let song_dialog = OpenDialog::builder()
            .transient_for_native(root)
            .launch(OpenDialogSettings::default())
            .forward(sender.input_sender(), |response| match response {
                OpenDialogResponse::Accept(path) => {
                    AppMsg::ChangeConfig(Box::new(move |mut config| {
                        config.sound_path = path;
                    }))
                }
                OpenDialogResponse::Cancel => AppMsg::Ignore,
            });
        let song_dialog = Rc::new(song_dialog);
        let song_dialog_ref = Rc::clone(&song_dialog);

        let widgets = view_output!();
        let popover_ref = widgets.popover.clone();

        widgets.song_btn.connect_clicked(move |_btn| {
            popover_ref.popdown();
            song_dialog_ref.emit(OpenDialogMsg::Open);
        });

        widgets.status_bar.push(0, &model.status_bar());

        ComponentParts { model, widgets }
    }

    fn post_view() {
        widgets.status_bar.remove_all(0);
        widgets.status_bar.push(0, &model.status_bar());
    }

    additional_fields! {
        song_dialog: Rc<Controller<OpenDialogInner<SingleSelection>>>,
    }

    view! {
        gtk::ApplicationWindow {
            set_title: Some("Pomo Oxide"),
            set_default_width: 350,
            set_icon_name: Some("pomodoro-indicator"),
            set_resizable: false,

            #[wrap(Some)]
            set_titlebar = &gtk::HeaderBar {
                pack_start = &gtk::MenuButton {
                    set_icon_name: ICON_CONFIG,

                    #[name = "popover"]
                    #[wrap(Some)]
                    set_popover = &gtk::Popover {
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            gtk::SpinButton {
                                set_tooltip_text: Some("Pomodoro Duration"),
                                set_range: (1.0, 180.0),
                                #[watch]
                                #[block_signal(toggle_handler_0)]
                                set_value: (model.config.pomodoro_time.as_secs() / 60) as f64,
                                set_increments: (1.0, 5.0),

                                connect_value_changed[sender] => move |btn| {
                                    let value = btn.value() as u64;
                                    sender.input(AppMsg::ChangeConfig(Box::new(move |mut config|
                                        config.pomodoro_time = Duration::from_secs(value * 60)
                                    )))
                                } @toggle_handler_0,
                            },

                            gtk::SpinButton {
                                set_tooltip_text: Some("Break Duration"),
                                set_range: (1.0, 180.0),
                                #[watch]
                                #[block_signal(toggle_handler_1)]
                                set_value: (model.config.break_time.as_secs() / 60) as f64,
                                set_increments: (1.0, 5.0),

                                connect_value_changed[sender] => move |btn| {
                                    let value = btn.value() as u64;
                                    sender.input(AppMsg::ChangeConfig(Box::new(move |mut config|
                                        config.break_time = Duration::from_secs(value * 60)
                                    )))
                                } @toggle_handler_1,
                            },

                            gtk::SpinButton {
                                set_tooltip_text: Some("Rest Duration"),
                                set_range: (1.0, 180.0),
                                #[watch]
                                #[block_signal(toggle_handler_2)]
                                set_value: (model.config.rest_time.as_secs() / 60) as f64,
                                set_increments: (1.0, 5.0),

                                connect_value_changed[sender] => move |btn| {
                                    let value = btn.value() as u64;
                                    sender.input(AppMsg::ChangeConfig(Box::new(move |mut config|
                                        config.rest_time = Duration::from_secs(value * 60)
                                    )))
                                } @toggle_handler_2,
                            },

                            gtk::SpinButton {
                                set_tooltip_text: Some("Rest Count - how many pomodoros until the break will be a rest."),
                                set_range: (1.0, 20.0),
                                #[watch]
                                #[block_signal(toggle_handler_3)]
                                set_value: model.config.rest_count as f64,
                                set_increments: (1.0, 2.0),

                                connect_value_changed[sender] => move |btn| {
                                    let value = btn.value() as u8;
                                    sender.input(AppMsg::ChangeConfig(Box::new(move |mut config|
                                        config.rest_count = value
                                    )))
                                } @toggle_handler_3,
                            },

                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 10,

                                #[name = "song_btn"]
                                gtk::Button::with_label("Change Sound") {
                                    #[watch]
                                    set_tooltip_text: Some(&format!("Current file: {:?}",
                                        model.config.sound_path.file_name().unwrap_or_default()
                                    )),
                                },

                                gtk::Button {
                                    set_icon_name: ICON_RESTART,
                                    set_tooltip_text: Some("Restore default configuration"),
                                    connect_clicked => AppMsg::ChangeConfig(Box::new(|config|
                                        *config = Config::default()
                                    )),
                                },
                            },

                        }
                    }
                },
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 10,

                gtk::Button {
                    #[watch]
                    set_icon_name: if model.running { ICON_PAUSE } else { ICON_START },
                    #[watch]
                    set_tooltip_text: Some(if model.running { "Pause" } else { "Start" }),
                    set_height_request: 50,
                    connect_clicked => AppMsg::Toggle(None),
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 10,
                    set_height_request: 40,
                    set_homogeneous: true,

                    gtk::Button {
                        set_icon_name: ICON_SKIP,
                        set_tooltip_text: Some("Skip"),
                        connect_clicked => AppMsg::Skip,
                    },

                    gtk::Button {
                        set_icon_name: ICON_RENEW,
                        #[watch]
                        set_tooltip_text: Some(&format!("Renew {}", model.state)),
                        connect_clicked => AppMsg::Renew,
                    },

                    gtk::Button {
                        set_icon_name: ICON_RESTART,
                        set_tooltip_text: Some("Restart"),
                        connect_clicked => AppMsg::Restart,
                    },
                },

                gtk::Label {
                    set_halign: gtk::Align::Center,
                    #[watch]
                    set_markup: &model.state_as_markup(),
                },

                gtk::Label {
                    set_halign: gtk::Align::Center,
                    #[watch]
                    set_markup: &min_as_markup(min_format(&model.timer)),
                    #[watch]
                    set_opacity: if model.running { 1.0 } else { 0.7 },
                },

                #[name = "status_bar"]
                gtk::Statusbar {
                    set_halign: gtk::Align::Center,
                },
            }

        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        self.clear_step_permission();

        match message {
            AppMsg::Ignore => return,
            AppMsg::Step => self.try_next_state(),
            AppMsg::Toggle(toggle) => self.toggle(toggle),
            AppMsg::Skip => self.next_state(),
            AppMsg::Renew => self.restart_state(),
            AppMsg::Restart => self.restart(),
            AppMsg::ChangeConfig(config_changer) => self.change_config(config_changer),
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
                    sender.input(AppMsg::Step);
                }
            });
        }
    }
}

struct AppModel {
    running: bool,
    timer: Duration,
    player: Player,
    song: Option<Song>,
    state: State,
    rest_counter: u8,
    pomodoro_count: usize,
    config: Config,
    config_file: PathBuf,
    /// Stores if a running thread has permission to send AppMsg::Step
    step_permission: Option<Arc<AtomicBool>>,
}

impl AppModel {
    fn new(config: Config, config_file: PathBuf) -> Self {
        let player = Player::new().expect("couldn't create audio player");
        let song = try_song(&config.sound_path);
        let state = State::default();
        let timer = state.duration(&config);
        Self {
            running: false,
            timer,
            state,
            rest_counter: 0,
            pomodoro_count: 0,
            config,
            config_file,
            step_permission: None,
            player,
            song,
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

impl AppModel {
    fn state_duration(&self) -> Duration {
        self.state.duration(&self.config)
    }

    fn state_as_markup(&self) -> String {
        self.state.as_markup(&self.config)
    }

    fn status_bar(&self) -> String {
        format!(
            "Completed: {}  -  Cycle ({}/{})",
            self.pomodoro_count, self.rest_counter, self.config.rest_count
        )
    }

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
                // avoid counting "skips" as complete pomodoros
                if self.timer.is_zero() {
                    self.pomodoro_count += 1;
                }

                if self.rest_counter + 1 >= self.config.rest_count {
                    State::Rest
                } else {
                    State::Break
                }
            }
            State::Break | State::Rest => {
                self.rest_counter += 1;
                self.rest_counter %= self.config.rest_count;
                State::Pomodoro
            }
        };
        if self.timer.is_zero() {
            if let Some(song) = self.song.as_ref() {
                self.player.play_song_now(song).unwrap();
            }
        }
        self.restart_state()
    }

    fn restart_state(&mut self) {
        self.timer = self.state_duration();
        info!("Starting {:?} - {}", &self.state, min_format(&self.timer));
    }

    fn restart(&mut self) {
        self.state = State::Pomodoro;
        self.rest_counter = 0;
        self.restart_state();
    }

    fn change_config(&mut self, config_changer: Box<dyn FnOnce(&mut Config) + Send>) {
        let previous_sound_path = self.config.sound_path.clone();

        config_changer(&mut self.config);

        if previous_sound_path != self.config.sound_path {
            match try_song(&self.config.sound_path) {
                Some(new_song) => self.song = Some(new_song),
                None => self.config.sound_path = previous_sound_path,
            };

            if let Some(song) = self.song.as_ref() {
                self.player.play_song_now(song).unwrap();
            }
        }

        info!("Saving config");
        (|| -> Result<_, Box<dyn std::error::Error>> {
            let encoded = serde_json::to_vec_pretty(&self.config)?;
            Ok(std::fs::write(&self.config_file, encoded)?)
        })()
        .unwrap_or_else(|e| warn!("Error saving config: {e}"));

        if self.config.rest_count <= self.rest_counter {
            self.restart();
        } else {
            self.restart_state();
        }
    }
}

enum AppMsg {
    Ignore,
    Step,
    Toggle(Option<bool>),
    Skip,
    Renew,
    Restart,
    ChangeConfig(Box<dyn FnOnce(&mut Config) + Send>),
}

impl std::fmt::Debug for AppMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ignore => write!(f, "Ignore"),
            Self::Step => write!(f, "Step"),
            Self::Toggle(arg0) => f.debug_tuple("Toggle").field(arg0).finish(),
            Self::Skip => write!(f, "Skip"),
            Self::Renew => write!(f, "Renew"),
            Self::Restart => write!(f, "Restart"),
            Self::ChangeConfig(_) => write!(f, "ChangeConfig(<closure>)"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Config {
    /// Time for each pomodoro.
    pomodoro_time: Duration,
    /// Time for each break.
    break_time: Duration,
    /// Time for each rest.
    rest_time: Duration,
    /// How many pomodoros until the break will be a rest.
    rest_count: u8,
    /// Sound file path
    sound_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pomodoro_time: Duration::from_secs(60 * 25),
            break_time: Duration::from_secs(60 * 5),
            rest_time: Duration::from_secs(60 * 20),
            rest_count: 4,
            sound_path: DEFAULT_SOUND.into(),
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

fn try_song(path: &PathBuf) -> Option<Song> {
    Song::from_file(path)
        .map_err(|e| warn!("Failed to open audio file: {e}"))
        .ok()
}
