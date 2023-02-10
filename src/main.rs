use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt};
use gtk::traits::WidgetExt;
use relm4::{send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};
use std::thread::sleep;
use std::time::Duration;

const SLEEP_STEP: Duration = Duration::from_millis(250);

fn main() {
    let model = AppModel::default();
    let app = RelmApp::new(model);
    app.run();

    // let config = Config::default();
    // let mut state = AppModel::default();

    // loop {
    //     println!("Starting Pomodoro - {}m", config.pomodoro_time.as_min());
    //     sleep(config.pomodoro_time / 60 / 5);

    //     state.pomodoro_count += 1;
    //     state.rest_counter += 1;

    //     if state.rest_counter % config.rest_count as usize == 0 {
    //         state.rest_counter = 0;

    //         println!("Starting Rest     - {}m", config.rest_time.as_min());
    //         sleep(config.rest_time / 60 / 5);
    //     } else {
    //         println!("Starting Break    - {}m", config.break_time.as_min());
    //         sleep(config.break_time / 60 / 5);
    //     }
    // }
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
        toggle_button.connect_clicked(move |_| send!(snd, AppMsg::Toggle));
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

#[derive(Debug, Default)]
struct AppModel {
    running: bool,
    timer: Duration,
    state: State,
    rest_counter: usize,
    pomodoro_count: usize,
    config: Config,
}

#[derive(Debug, Default)]
enum State {
    Pomodoro,
    Break,
    #[default] // After start, change to pomodoro
    Rest,
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = ();
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: Self::Msg, _: &Self::Components, _sender: Sender<Self::Msg>) -> bool {
        dbg!(&msg);

        // let timer = components.timer.

        match msg {
            AppMsg::Start => self.toggle(true),
            AppMsg::Pause => self.toggle(false),
            AppMsg::Toggle => self.toggle(!self.running),
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

        true
    }
}

impl AppModel {
    fn toggle(&mut self, running: bool) {
        self.running = running
    }
    fn step_sleep(&mut self) {
        let duration = SLEEP_STEP.min(self.timer);
        self.timer -= duration;
        sleep(duration)
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
        self.timer = match self.state {
            State::Pomodoro => self.config.pomodoro_time,
            State::Break => self.config.break_time,
            State::Rest => self.config.rest_time,
        };
        println!(
            "Starting {:?} - {}",
            &self.state,
            self.timer.as_min_format()
        );
        self.toggle(true);
    }
}

#[allow(unused)]
#[derive(Debug)]
enum AppMsg {
    Start,
    Pause,
    Toggle,
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
