use crate::lang::Lang;
use std::borrow::Cow;
pub fn text_for_speech(text: &str, _lang: Lang) -> Cow<'_, str> {
    Cow::Borrowed(text)
}
