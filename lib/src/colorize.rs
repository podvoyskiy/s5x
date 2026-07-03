pub trait Colorize {
    fn red(&self)     -> String;
    fn _green(&self)  -> String;
    fn _cyan(&self)   -> String;
    fn _yellow(&self) -> String;
}

impl<T: AsRef<str>> Colorize for T {
    fn red(&self)     -> String { format!("\x1b[31m{}\x1b[0m", self.as_ref()) }
    fn _green(&self)  -> String { format!("\x1b[32m{}\x1b[0m", self.as_ref()) }
    fn _cyan(&self)   -> String { format!("\x1b[36m{}\x1b[0m", self.as_ref()) }
    fn _yellow(&self) -> String { format!("\x1b[33m{}\x1b[0m", self.as_ref()) }
}