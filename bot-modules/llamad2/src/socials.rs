use serenity::Error;
use serenity::all::{
    CommandInteraction,
    Context,
    CreateCommand,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    EmojiId,
    ReactionType,
};

const YOUTUBE_ICON: EmojiId = EmojiId::new(1_391_908_911_136_641_175);
const TWITCH_ICON: EmojiId = EmojiId::new(1_391_913_042_916_413_633);
const TWITTER_ICON: EmojiId = EmojiId::new(1_391_915_596_584_587_335);
const TICTOK_ICON: EmojiId = EmojiId::new(1_391_912_807_716_622_397);

pub struct Socials;

impl Socials {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> Result<(), Error> {
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
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("socials")
            .description("Shows all of Llama's other socials")
    }
}
