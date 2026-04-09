//! # Orbit Core
//!
//! This crate provides core capabilities for the Orbit system.

pub mod config;

#[cfg(test)]
mod example;

#[cfg(test)]
mod test_config_loading;

#[cfg(test)]
mod test_project_config;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
