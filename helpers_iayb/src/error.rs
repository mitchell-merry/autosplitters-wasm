use std::fmt;
use std::fmt::{Debug, Display};

#[derive(Debug)]
pub struct SimpleError {
    message: String,
}

impl SimpleError {
    pub fn wrap<T: Display>(message: T, child: Box<dyn std::error::Error>) -> SimpleError {
        SimpleError::from(&format!("{}: {}", message, child))
    }
}

impl std::error::Error for SimpleError {}

impl fmt::Display for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<&str> for SimpleError {
    fn from(message: &str) -> Self {
        SimpleError {
            message: message.to_string(),
        }
    }
}
impl From<&String> for SimpleError {
    fn from(message: &String) -> Self {
        SimpleError {
            message: message.clone(),
        }
    }
}
