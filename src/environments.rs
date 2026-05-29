#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentOption {
    pub display_name: String,
    pub cli_value: Option<String>,
}

impl EnvironmentOption {
    pub fn no_environment() -> Self {
        Self {
            display_name: "No environment".to_string(),
            cli_value: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EnvironmentOption;

    #[test]
    fn no_environment_option_is_explicit() {
        let env = EnvironmentOption::no_environment();

        assert_eq!(env.display_name, "No environment");
        assert_eq!(env.cli_value, None);
    }
}
