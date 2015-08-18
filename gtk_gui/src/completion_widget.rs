use gtk::signal;
use gtk::traits::*;
use gtk;

pub struct CompletionWidget {
    pub widget: gtk::Entry,
}

impl CompletionWidget {
    pub fn new<T: gtk::WidgetTrait>(widget: &T) -> gtk::Box {

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0).unwrap();
        container.set_border_width(10);
        // container.set_default_size((widget.get_allocated_width() as f32 * 0.8).round() as i32, 200);

        // let listview = gtk::Listview();

        let entry = gtk::Entry::new().unwrap();
        entry.set_has_frame(true);
        entry.set_halign(gtk::Align::Center);
        entry.set_valign(gtk::Align::Start);

        container.pack_start(&entry, true, true, 0);

        container.show_all();
        container
    }
}
