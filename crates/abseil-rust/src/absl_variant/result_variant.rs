//! ResultVariant - A variant type for results with detailed error information.

use alloc::string::String;

/// A variant type for results with detailed error information.
#[derive(Clone, Debug, PartialEq)]
pub enum ResultVariant<T, E> {
    /// Success value.
    Ok(T),
    /// Error with message.
    Err(E, String),
}

impl<T, E> ResultVariant<T, E> {
    /// Creates an Ok variant.
    pub fn ok(value: T) -> Self {
        ResultVariant::Ok(value)
    }

    /// Creates an Err variant with a message.
    pub fn err(error: E, message: String) -> Self {
        ResultVariant::Err(error, message)
    }

    /// Returns true if this is Ok.
    pub fn is_ok(&self) -> bool {
        matches!(self, ResultVariant::Ok(_))
    }

    /// Returns true if this is Err.
    pub fn is_err(&self) -> bool {
        matches!(self, ResultVariant::Err(_, _))
    }

    /// Returns the Ok value if present.
    pub fn as_ok(&self) -> Option<&T> {
        match self {
            ResultVariant::Ok(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the Err value if present.
    pub fn as_err(&self) -> Option<(&E, &str)> {
        match self {
            ResultVariant::Err(e, msg) => Some((e, msg)),
            _ => None,
        }
    }

    /// Maps the Ok value.
    pub fn map<U, F>(self, f: F) -> ResultVariant<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ResultVariant::Ok(v) => ResultVariant::Ok(f(v)),
            ResultVariant::Err(e, msg) => ResultVariant::Err(e, msg),
        }
    }

    /// Maps the Err value.
    pub fn map_err<F, E2>(self, f: F) -> ResultVariant<T, E2>
    where
        F: FnOnce(E, String) -> (E2, String),
    {
        match self {
            ResultVariant::Ok(v) => ResultVariant::Ok(v),
            ResultVariant::Err(e, msg) => {
                let (e2, msg2) = f(e, msg);
                ResultVariant::Err(e2, msg2)
            }
        }
    }

    /// Converts to a standard Result.
    pub fn to_result(self) -> Result<T, (E, String)> {
        match self {
            ResultVariant::Ok(v) => Ok(v),
            ResultVariant::Err(e, msg) => Err((e, msg)),
        }
    }

    /// Creates from a standard Result.
    pub fn from_result(r: Result<T, E>, err_msg: impl FnOnce() -> String) -> Self {
        match r {
            Ok(v) => ResultVariant::Ok(v),
            Err(e) => ResultVariant::Err(e, err_msg()),
        }
    }
}

impl<T: Default, E> ResultVariant<T, E> {
    /// Returns the Ok value or a default.
    pub fn unwrap_or_default(self) -> T {
        match self {
            ResultVariant::Ok(v) => v,
            ResultVariant::Err(_, _) => T::default(),
        }
    }
}

impl<T: Clone, E> ResultVariant<T, E> {
    /// Returns the Ok value or a clone of the provided default.
    pub fn unwrap_or_clone(&self, default: &T) -> T {
        match self {
            ResultVariant::Ok(v) => v.clone(),
            ResultVariant::Err(_, _) => default.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_variant_ok() {
        let rv: ResultVariant<i32, String> = ResultVariant::ok(42);
        assert!(rv.is_ok());
        assert!(!rv.is_err());
        assert_eq!(rv.as_ok(), Some(&42));
    }

    #[test]
    fn test_result_variant_err() {
        let rv: ResultVariant<i32, String> = ResultVariant::err("error".to_string(), "failed".to_string());
        assert!(!rv.is_ok());
        assert!(rv.is_err());
        assert_eq!(rv.as_err(), Some(&("error".to_string(), "failed")));
    }

    #[test]
    fn test_result_variant_map() {
        let rv: ResultVariant<i32, String> = ResultVariant::ok(42);
        let mapped = rv.map(|v| v * 2);
        assert!(mapped.is_ok());
        assert_eq!(mapped.as_ok(), Some(&84));
    }

    #[test]
    fn test_result_variant_map_err() {
        let rv: ResultVariant<i32, String> = ResultVariant::err("error".to_string(), "failed".to_string());
        let mapped = rv.map_err(|e, msg| (e.to_uppercase(), msg.to_uppercase()));
        assert!(mapped.is_err());
        assert_eq!(mapped.as_err(), Some(&("ERROR".to_string(), "FAILED".to_string())));
    }

    #[test]
    fn test_result_variant_to_result() {
        let rv: ResultVariant<i32, String> = ResultVariant::ok(42);
        assert_eq!(rv.to_result(), Ok(42));

        let rv: ResultVariant<i32, String> = ResultVariant::err("error".to_string(), "failed".to_string());
        assert_eq!(rv.to_result(), Err(("error".to_string(), "failed".to_string())));
    }

    #[test]
    fn test_result_variant_from_result() {
        let rv: ResultVariant<i32, String> = ResultVariant::from_result(Ok(42), || "default".to_string());
        assert!(rv.is_ok());

        let rv: ResultVariant<i32, String> = ResultVariant::from_result(Err("error".to_string()), || "default".to_string());
        assert!(rv.is_err());
    }

    #[test]
    fn test_result_variant_unwrap_or_default() {
        let rv: ResultVariant<i32, String> = ResultVariant::ok(42);
        assert_eq!(rv.unwrap_or_default(), 42);

        let rv: ResultVariant<i32, String> = ResultVariant::err("error".to_string(), "failed".to_string());
        assert_eq!(rv.unwrap_or_default(), 0);
    }
}
