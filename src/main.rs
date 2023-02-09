use std::thread::sleep;
use std::time::Duration;

fn main() {
    let config = Config::default();
    let mut state = State::default();

    loop {
        println!("Starting Pomodoro - {}m", config.pomodoro_time.as_min());
        sleep(config.pomodoro_time / 60 / 5);

        state.pomodoro_count += 1;
        state.rest_counter += 1;

        if state.rest_counter % config.rest_count as usize == 0 {
            state.rest_counter = 0;

            println!("Starting Rest     - {}m", config.rest_time.as_min());
            sleep(config.rest_time / 60 / 5);
        } else {
            println!("Starting Break    - {}m", config.break_time.as_min());
            sleep(config.break_time / 60 / 5);
        }
    }
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

#[derive(Debug, Default)]
struct State {
    rest_counter: usize,
    pomodoro_count: usize,
}

trait AsMinute {
    fn as_min(&self) -> u64;
}

impl AsMinute for Duration {
    fn as_min(&self) -> u64 {
        self.as_secs() / 60
    }
}
