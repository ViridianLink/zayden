use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, EmojiId, ReactionType,
};

use crate::LLAMA_GUILD;

const YOUTUBE_ICON: EmojiId = EmojiId::new(1391908911136641175);
const TWITCH_ICON: EmojiId = EmojiId::new(1391913042916413633);
const TWITTER_ICON: EmojiId = EmojiId::new(1391915596584587335);
const TICTOK_ICON: EmojiId = EmojiId::new(1391912807716622397);

pub struct Socials;

impl Socials {
    pub async fn run(ctx: &Context, interaction: &CommandInteraction) {
        if interaction
            .guild_id
            .is_none_or(|guild| guild != LLAMA_GUILD)
            || interaction.user.bot()
        {
            return;
        }

        let content = format!(
            "Here are all of my socials!

            - {} YouTube - <https://www.youtube.com/@LlamaD2>
            - {} Twitch - <https://www.twitch.tv/llamad2>
            - {} Twitter/X - <https://x.com/LlamaOnD2>
            - {} TikTok - <https://www.tiktok.com/@llamad2>",
            ReactionType::from(YOUTUBE_ICON),
            ReactionType::from(TWITCH_ICON),
            ReactionType::from(TWITTER_ICON),
            ReactionType::from(TICTOK_ICON)
        );

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(content),
                ),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("socials").description("Shows all of Llama's other socials")
    }
}
