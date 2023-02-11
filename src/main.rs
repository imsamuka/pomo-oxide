use log::info;
use relm4::gtk;
use relm4::gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt};
use relm4::{send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};
use std::sync::{Arc, atomic, atomic::AtomicBool};
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

#[allow(unused)]
struct AppWidgets {
    window: gtk::ApplicationWindow,
    vbox: gtk::Box,
    toggle_button: gtk::Button,
    skip_button: gtk::Button,
    renew_button: gtk::Button,
    restart_button: gtk::Button,
    timer_label: gtk::Label,
    state_label: gtk::Label,
}

impl Widgets<AppModel, ()> for AppWidgets {
    type Root = gtk::ApplicationWindow;

    fn init_view(
        model: &AppModel,
        _components: &<AppModel as Model>::Components,
        sender: Sender<<AppModel as Model>::Msg>,
    ) -> Self {
        let window = gtk::ApplicationWindow::builder()
            .title("Pomo Oxide")
            .default_width(350)
            .build();
        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(10)
            .build();
        vbox.set_margin_all(10);

        let hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .height_request(35)
            .spacing(10)
            .build();

        // let config_button = gtk::MenuButton;

        let toggle_button = gtk::Button::builder()
            .icon_name(ICON_START)
            .tooltip_text("Start")
            .height_request(45)
            .build();
        let skip_button = gtk::Button::builder()
            .icon_name(ICON_SKIP)
            .tooltip_text("Skip")
            .hexpand(true)
            .build();
        let renew_button = gtk::Button::builder()
            .icon_name(ICON_RENEW)
            .tooltip_text(&format!("Renew {}", model.state))
            .hexpand(true)
            .build();
        let restart_button = gtk::Button::builder()
            .icon_name(ICON_RESTART)
            .tooltip_text("Restart")
            .hexpand(true)
            .build();

        let timer_label = gtk::Label::new(None);
        timer_label.set_halign(gtk::Align::Center);
        timer_label.set_markup(&min_as_markup(min_format(&model.timer)));

        let state_label = gtk::Label::new(None);
        state_label.set_halign(gtk::Align::Center);
        state_label.set_markup(&model.state.as_markup(&model.config));

        // Connect the widgets
        window.set_child(Some(&vbox));
        hbox.append(&skip_button);
        hbox.append(&renew_button);
        hbox.append(&restart_button);
        vbox.append(&toggle_button);
        vbox.append(&hbox);
        vbox.append(&state_label);
        vbox.append(&timer_label);

        // Connect events
        let snd = sender.clone();
        toggle_button.connect_clicked(move |_| send!(snd, AppMsg::Toggle(None)));
        let snd = sender.clone();
        skip_button.connect_clicked(move |_| send!(snd, AppMsg::Skip));
        let snd = sender.clone();
        renew_button.connect_clicked(move |_| send!(snd, AppMsg::Renew));
        restart_button.connect_clicked(move |_| send!(sender, AppMsg::Restart));

        Self {
            window,
            vbox,
            toggle_button,
            skip_button,
            renew_button,
            restart_button,
            timer_label,
            state_label,
        }
    }

    fn root_widget(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(&mut self, model: &AppModel, _sender: Sender<<AppModel as Model>::Msg>) {
        if model.running {
            self.toggle_button.set_tooltip_text(Some("Pause"));
            self.toggle_button.set_icon_name(ICON_PAUSE)
        } else {
            self.toggle_button.set_tooltip_text(Some("Start"));
            self.toggle_button.set_icon_name(ICON_START)
        }
        self.renew_button
            .set_tooltip_text(Some(&format!("Renew {}", model.state)));
        self.timer_label
            .set_markup(&min_as_markup(min_format(&model.timer)));
        self.state_label
            .set_markup(&model.state.as_markup(&model.config));
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
            AppMsg::Restart => {
                self.state = State::Pomodoro;
                self.rest_counter = 0;
                self.restart_state();
            }
            AppMsg::ChangeConfig(config) => {
                self.config = *config;
                self.restart_state()
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

                if self.rest_counter % self.config.rest_count as usize == 0 {
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
}

#[allow(unused)]
#[derive(Debug)]
enum AppMsg {
    Step,
    Toggle(Option<bool>),
    Skip,
    Renew,
    Restart,
    ChangeConfig(Box<Config>),
}

#[derive(Debug)]
struct Config {
    /// Time for each pomodoro.
    pomodoro_time: Duration,
    /// Time for each break after a pomodoro.
    break_time: Duration,
    /// Time for each rest after `Config::pomodoro_count` pomodoros.
    rest_time: Duration,
    /// How many pomodoros until a rest.
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
