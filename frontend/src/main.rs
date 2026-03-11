mod domain;
mod application;
mod infrastructure;
mod presentation;

fn main() {
    dioxus::launch(presentation::App);
}
