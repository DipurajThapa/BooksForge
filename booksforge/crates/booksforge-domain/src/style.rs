use serde::{Deserialize, Serialize};

/// Which dash character to use when the writer types "--".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EmDash {
    #[default]
    Em, // —
    En,     // –
    Hyphen, // -
}

/// Typographic quote style applied by the Copyeditor Agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QuoteStyle {
    #[default]
    Smart, // "curly"
    Straight, // "straight"
}

/// How three-dot ellipsis is rendered in exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EllipsisForm {
    #[default]
    SingleGlyph, // …
    ThreeDots, // ...
}

/// Per-project mechanical-style choices.
///
/// The Copyeditor Agent reads this; the user edits it from Project Settings.
/// Stored as a singleton row in the `style_book` SQLite table and mirrored
/// into `manifest.toml [style_book]` for portability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleBook {
    pub em_dash: EmDash,
    pub oxford_comma: bool,
    pub quote_style: QuoteStyle,
    pub spaces_after_period: u8,
    pub ellipsis_form: EllipsisForm,
    pub spelling_locale: String,
    pub capitalize_after_colon: bool,
    pub bold_emphasis_allowed: bool,
}

impl Default for StyleBook {
    fn default() -> Self {
        Self {
            em_dash: EmDash::Em,
            oxford_comma: true,
            quote_style: QuoteStyle::Smart,
            spaces_after_period: 1,
            ellipsis_form: EllipsisForm::SingleGlyph,
            spelling_locale: "en-US".to_owned(),
            capitalize_after_colon: false,
            bold_emphasis_allowed: false,
        }
    }
}
