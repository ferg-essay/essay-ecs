pub struct Error {
    msg: String
}

impl Error {
    #[inline]
    pub fn message(&self) -> &str {
        &self.msg
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self {
            msg: value.to_string()
        }
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self {
            msg: value
        }
    }
}

impl std::fmt::Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

pub type Result<V, E=Error> = std::result::Result<V, E>;
