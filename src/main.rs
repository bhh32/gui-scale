mod helpers;

use std::sync::{Arc, Mutex};

use cushy::{
    figures::units::Px, 
    value::{Dynamic, Source}, 
    widget::MakeWidget, 
    widgets::{button, checkbox::CheckboxState, input::{Input, InputValue}, Button}, 
    Run
};

use helpers::*;

fn main() -> cushy::Result {
    // Create storage for user to enter a name.
    let name: Dynamic<String> = Dynamic::default();

    // Create the label by using 'map_each' to format the name, first checking
    // if it's empty.
    let greeting = name.map_each(|name| {
        let name = if name.is_empty() { "World" } else { name };
        format!("Hello, {name}!")
    })
    .width(Px::new(300));

    // Create the input widget with a placeholder.
    let name_input: Input = name.into_input().placeholder("Name");

    let allow_ssh = Arc::new(Mutex::new(true));
    let ssh_clone = allow_ssh.clone();
    let allow_ssh_checkbox = Button::new("Allow SSH")
        .on_click(move |_| {
            let mut ssh = ssh_clone.lock().unwrap();
            if *ssh {
                *ssh = false;
            } else {
                *ssh = true;
            }
        })
        .into_checkbox(Dynamic::new(CheckboxState::Checked));

    let ssh = match allow_ssh.lock() {
        Ok(ssh) => *ssh,
        Err(_) => false,
    };

    // Create allow routes checkbox
    let accept_routes = Arc::new(Mutex::new(true));
    let routes_clone = accept_routes.clone();
    let accept_routes_checkbox = Button::new("Accept Routes")
        .on_click(move |_| {
            let mut routes = routes_clone.lock().unwrap();
            if *routes {
                *routes = false;
            } else {
                *routes = true;
            }
        })
        .into_checkbox(Dynamic::new(CheckboxState::Checked));

    let routes = match accept_routes.lock() {
        Ok(routes) => *routes,
        Err(_) => false,
    };

    // Up button
    let up_btn: Button = button::Button::new("Tailscale Up")
        .on_click(move |_| {
            tailscale_int_up(true, ssh, routes);
        });
    
    // Down button
    let down_btn: Button = button::Button::new("Tailscale Down")
        .on_click(|_| {
            tailscale_int_up(false, false, false);
        });

    let up_down_btns = up_btn.and(down_btn).into_columns();
    let allows = allow_ssh_checkbox.and(accept_routes_checkbox).into_columns();
    // Stack the widgets as rows, and then run the app.
    name_input.and(greeting).and(up_down_btns).and(allows).into_rows().centered().align_top().run()
}
