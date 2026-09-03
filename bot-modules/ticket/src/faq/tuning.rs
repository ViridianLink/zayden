use zayden_app::config::FaqSettingsRow;

#[derive(Debug, Clone, Copy)]
pub struct AnswerTuning {
    pub max_tokens: u32,
    pub temperature: f32,
}

impl AnswerTuning {
    pub const DEFAULT_MAX_TOKENS: u32 = 500;
    pub const DEFAULT_TEMPERATURE: f32 = 0.2;

    #[must_use]
    pub fn from_settings(row: &FaqSettingsRow) -> Self {
        Self {
            max_tokens: u32::try_from(row.answer_max_tokens.clamp(64, 4096))
                .unwrap_or(Self::DEFAULT_MAX_TOKENS),
            temperature: row.answer_temperature.clamp(0.0, 2.0),
        }
    }
}
