use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
pub struct Spinner {
    pub pb: ProgressBar,
}

impl Spinner {
    pub fn new(message: String) -> Self {
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_style(
            ProgressStyle::with_template("{spinner:.blue} {msg}")
                .unwrap()
                .tick_strings(&[
                    "▹▹▹▹▹",
                    "▸▹▹▹▹",
                    "▹▸▹▹▹",
                    "▹▹▸▹▹",
                    "▹▹▹▸▹",
                    "▹▹▹▹▸",
                    "▪▪▪▪▪",
                ]),
        );
        pb.set_message(message);
        Self { pb }
    }

    pub fn message(self, message: String) {
        self.pb.set_message(message);
    }

    pub fn success(self, message: String) {
        self.pb.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg}")
                .unwrap()
                .tick_strings(&["✔"]),
        );
        self.pb.finish_with_message(message);
    }

    pub fn error(self, message: String) {
        self.pb.set_style(
            ProgressStyle::with_template("{spinner:.red} {msg}")
                .unwrap()
                .tick_strings(&["✖"]),
        );
        self.pb.finish_with_message(message);
    }
}
