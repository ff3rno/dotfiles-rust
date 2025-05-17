use colored::*;
use std::fmt::Display;

pub fn success<T: Display>(text: T) -> impl Display {
    text.to_string().green()
}

pub fn error<T: Display>(text: T) -> impl Display {
    text.to_string().red()
}

pub fn warning<T: Display>(text: T) -> impl Display {
    text.to_string().yellow()
}

pub fn info<T: Display>(text: T) -> impl Display {
    text.to_string().cyan()
}

pub fn highlight<T: Display>(text: T) -> impl Display {
    text.to_string().blue()
}

pub fn header<T: Display>(text: T) -> impl Display {
    text.to_string().magenta().bold()
}

pub fn dry_run<T: Display>(text: T) -> impl Display {
    text.to_string().yellow().bold()
}

pub fn path<T: Display>(text: T) -> impl Display {
    text.to_string().blue().bold()
}

pub fn version<T: Display>(text: T) -> impl Display {
    text.to_string().green().bold()
}