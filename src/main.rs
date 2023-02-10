use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt};
use gtk::traits::WidgetExt;
use relm4::{send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::time::Duration;

const SLEEP_STEP: Duration = Duration::from_millis(250);

fn main() {
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
    restart_button: gtk::Button,
    label: gtk::Label,
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
            .default_width(400)
            .default_height(350)
            .build();
        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(10)
            .build();
        vbox.set_margin_all(10);

        let hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .build();

        /* Icons:
        preferences-system-symbolic applications-system-symbolic
        media-playback-start-symbolic media-playback-pause-symbolic
        media-skip-forward-symbolic go-first-symbolic media-skip-forward-symbolic.symbolic go-next-symbolic
        media-refresh-symbolic go-previous-symbolic object-rotate-left-symbolic system-reboot-symbolic
        */

        // let config_button = gtk::MenuButton;

        let toggle_button = gtk::Button::builder()
            .icon_name("media-playback-start-symbolic")
            .tooltip_text("Start")
            .height_request(35)
            .build();
        let skip_button = gtk::Button::builder()
            .icon_name("go-first-symbolic")
            .tooltip_text("Skip")
            .hexpand(true)
            .build();
        let restart_button = gtk::Button::builder()
            .icon_name("object-rotate-left-symbolic")
            .tooltip_text("Restart")
            .hexpand(true)
            .build();

        let label = gtk::Label::new(Some(&model.timer.as_min_format()));
        label.set_halign(gtk::Align::Center);
        label.set_margin_all(10);

        // Connect the widgets
        window.set_child(Some(&vbox));
        vbox.append(&toggle_button);
        hbox.append(&skip_button);
        hbox.append(&restart_button);
        vbox.append(&hbox);
        vbox.append(&label);

        // Connect events
        let snd = sender.clone();
        toggle_button.connect_clicked(move |_| send!(snd, AppMsg::Toggle(None)));
        let snd = sender.clone();
        skip_button.connect_clicked(move |_| send!(snd, AppMsg::Skip));
        restart_button.connect_clicked(move |_| send!(sender, AppMsg::Restart));

        Self {
            window,
            vbox,
            toggle_button,
            skip_button,
            restart_button,
            label,
        }
    }

    fn root_widget(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(&mut self, model: &AppModel, _sender: Sender<<AppModel as Model>::Msg>) {
        if model.running {
            self.toggle_button.set_tooltip_text(Some("Pause"));
            self.toggle_button
                .set_icon_name("media-playback-pause-symbolic")
        } else {
            self.toggle_button.set_tooltip_text(Some("Start"));
            self.toggle_button
                .set_icon_name("media-playback-start-symbolic")
        }

        self.label.set_label(&model.timer.as_min_format());
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

impl State {
    fn duration(&self, config: &Config) -> Duration {
        match self {
            State::Pomodoro => config.pomodoro_time,
            State::Break => config.break_time,
            State::Rest => config.rest_time,
        }
    }
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = ();
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: Self::Msg, _: &Self::Components, sender: Sender<Self::Msg>) -> bool {
        dbg!(&msg);

        self.clear_step_permission();

        match msg {
            AppMsg::Step => self.try_next_state(),
            AppMsg::Toggle(toggle) => self.toggle(toggle),
            AppMsg::Skip => self.next_state(),
            AppMsg::RestartCurrent => self.restart_state(),
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
        println!(
            "Starting {:?} - {}",
            &self.state,
            self.timer.as_min_format()
        );
        self.toggle(Some(true));
    }
}

#[allow(unused)]
#[derive(Debug)]
enum AppMsg {
    Step,
    Toggle(Option<bool>),
    Skip,
    RestartCurrent,
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

trait AsMinute {
    fn as_min(&self) -> u64;
    fn as_min_format(&self) -> String;
}

impl AsMinute for Duration {
    fn as_min(&self) -> u64 {
        self.as_secs() / 60
    }

    fn as_min_format(&self) -> String {
        format!("{}:{:02}", self.as_secs() / 60, self.as_secs() % 60)
    }
}
