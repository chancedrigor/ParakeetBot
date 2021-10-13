use crate::bot;
use crate::Result;

use super::SlashCommand;

#[derive(Debug, Default)]
pub struct Dc;

impl SlashCommand for Dc {
    fn name(&self) -> &str {
        "dc"
    }

    fn description(&self) -> &str {
        "I didn't want to stay here anyway!"
    }

    fn options(&self) -> Vec<serenity::builder::CreateApplicationCommandOption> {
        vec![]
    }
}
