use clap::ValueEnum;
use colored::Colorize;
use serde::Serialize;
use tabled::{settings::Style, Table, Tabled};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Table,
    Json,
}

pub fn print_json<T: Serialize>(value: &T) {
    let json = serde_json::to_string_pretty(value).unwrap();
    println!("{}", json);
}

pub fn print_table<T: Tabled>(items: Vec<T>) {
    let table = Table::new(items).with(Style::rounded()).to_string();
    println!("{}", table);
}

pub fn print_single<T: Tabled>(item: T) {
    print_table(vec![item]);
}

pub fn print_success(msg: &str) {
    println!("{} {}", "✓".green(), msg);
}

pub fn print_created<T: Serialize + Tabled>(item: T, format: Format) {
    match format {
        Format::Json => print_json(&item),
        Format::Table => {
            print_success("Created");
            print_single(item);
        }
    }
}

pub fn print_item<T: Serialize + Tabled>(item: T, format: Format) {
    match format {
        Format::Json => print_json(&item),
        Format::Table => print_single(item),
    }
}

pub fn print_items<T: Serialize + Tabled>(items: Vec<T>, format: Format) {
    match format {
        Format::Json => print_json(&items),
        Format::Table => {
            if items.is_empty() {
                println!("No results");
            } else {
                print_table(items);
            }
        }
    }
}
