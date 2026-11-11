use std::fmt;

/// A forge access token.
///
/// Held in memory only. It has no `Display` and no `Serialize`, and its
/// `Debug` is redacted, so it cannot reach the UI, a log line or a file by
/// accident. Only this crate can read the secret.
pub struct Token(String);

impl Token {
    pub(crate) fn new(secret: String) -> Self {
        Self(secret)
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_shows_the_secret() {
        let token = Token::new("gho_s3cret".to_string());

        let shown = format!("{token:?} {token:#?}");

        assert!(!shown.contains("gho_s3cret"), "leaked: {shown}");
        assert!(shown.contains("redacted"));
    }

    /// Compiles only while `Token` implements neither `Serialize` nor
    /// `Display`: if either is added, `some_item` resolves two ways and the
    /// build fails. Without them the token can't be sent to the UI, written
    /// to a file through serde, or formatted into a log line.
    #[test]
    fn token_cannot_be_serialized_or_displayed() {
        trait AmbiguousIfImpl<A> {
            fn some_item() {}
        }
        impl<T: ?Sized> AmbiguousIfImpl<()> for T {}

        struct ViaSerialize;
        impl<T: ?Sized + serde::Serialize> AmbiguousIfImpl<ViaSerialize> for T {}
        struct ViaDisplay;
        impl<T: ?Sized + fmt::Display> AmbiguousIfImpl<ViaDisplay> for T {}

        <Token as AmbiguousIfImpl<_>>::some_item();
    }
}
