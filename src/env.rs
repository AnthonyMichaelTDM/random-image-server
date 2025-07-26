use std::env::VarError;

pub trait EnvBackend {
    /// Read an environment variable
    ///
    /// # Errors
    ///
    /// Returns an error if the variable is not set
    fn var(&self, var: &str) -> Result<String, VarError>;

    /// Set an environment variable
    fn set_var(&mut self, var: &str, value: &str);

    /// Remove an environment variable
    fn remove(&mut self, var: &str);
}

pub(crate) struct StdEnvBackend;
impl EnvBackend for StdEnvBackend {
    fn var(&self, var: &str) -> Result<String, VarError> {
        std::env::var(var)
    }

    fn set_var(&mut self, var: &str, value: &str) {
        unsafe { std::env::set_var(var, value) };
    }

    fn remove(&mut self, var: &str) {
        unsafe { std::env::remove_var(var) };
    }
}

#[derive(Default)]
pub struct MockEnvBackend {
    vars: std::collections::HashMap<String, String>,
}

impl EnvBackend for MockEnvBackend {
    fn var(&self, var: &str) -> Result<String, VarError> {
        self.vars.get(var).cloned().ok_or(VarError::NotPresent)
    }

    fn set_var(&mut self, var: &str, value: &str) {
        self.vars.insert(var.to_string(), value.to_string());
    }

    fn remove(&mut self, var: &str) {
        self.vars.remove(var);
    }
}
