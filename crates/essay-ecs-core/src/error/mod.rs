use std::error;

pub struct Error {
    msg: String,
    source: Option<Box<dyn error::Error + Send + Sync>>
}

impl Error {
    #[inline]
    pub fn new(msg: &str) -> Self {
        Error {
            msg: msg.to_string(),
            source: None,
        }
    }

    #[inline]
    pub fn other<E>(error: E) -> Self
    where 
        E: Into<Box<dyn error::Error + Send + Sync>>
    {
        let error = error.into();

        Error {
            msg: format!("{}", error),
            source: Some(error),
        }
    }

    #[inline]
    pub fn other_loc<E>(error: E, loc: &str) -> Self
    where 
        E: Into<Box<dyn error::Error + Send + Sync>>
    {
        let error = error.into();

        Error {
            msg: format!("{}\n\tat {}", error, loc),
            source: Some(error),
        }
    }

    pub fn rethrow(self, loc: &str) -> Self {
        Error {
            msg: format!("{}{}", self.message(), loc),
            ..self
        }
    }

    #[inline]
    pub fn message(&self) -> &str {
        &self.msg
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self {
            msg: value.to_string(),
            source: None,
        }
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self {
            msg: value,
            source: None,
        }
    }
}

impl std::fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::fmt::Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &_)
    }
}

pub type Result<V, E=Error> = std::result::Result<V, E>;

#[allow(unused_macros)]
macro_rules! error_loc {
    ($($param:expr),*) => {
        $crate::error::Error::new(&format!("{} in {}\n\tat {}:{}:{}", 
            format_args!($($param,)*), 
            module_path!(),
            file!(), 
            line!(), 
            column!()
        ))
    }
}

#[allow(unused_macros)]
macro_rules! rethrow {
    ($err:expr, $($param:expr),*) => {
        $err.rethrow(&format_args!($($param,)*)), 
    }
}

#[cfg(test)]
mod test {
    // use super::Error;

    #[test]
    fn test_error() {
        /*
        let error = Error::other("test");
        println!("Error {:?}", error.source);
        */
    }

    #[test]
    fn test_error_log() {
        /*
        assert_eq!(
            "My message 13 in essay_ecs_core::error::test\n\tat crates/essay-ecs-core/src/error/mod.rs:102:13",
            error_loc!("My message {}", 13).message(),
        );
        */
    }
}