use teloxide::Bot;
use std::env;

pub fn build_bot() -> Bot {
    let token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN must be set");
    Bot::new(token)
}
