use eframe::egui::{Id, Ui};

macro_rules! ID {
    ($ex:expr ) => {
        concat!("AvailableRectCalculator_", $ex)
    };
}

pub fn width<R>(ui: &mut eframe::egui::Ui, add_contents: impl FnOnce(&mut Ui, f32) -> R) -> R {
    const WIDTH_ID: &str = ID!("cal_target_width");
    // this frame target width
    //   == this frame initial max rect width - last frame others width
    let id_cal_target_size = Id::new(WIDTH_ID);
    let this_init_max_width = ui.max_rect().width();
    let last_others_width = ui.data(|data| {
        data.get_temp(id_cal_target_size)
            .unwrap_or(this_init_max_width)
    });
    // this is the total available space for expandable widgets, you can divide
    // it up if you have multiple widgets to expand, even with different ratios.
    let this_target_width = this_init_max_width - last_others_width;
    let res = add_contents(ui, this_target_width);

    // this frame others width
    //   == this frame final min rect width - this frame target width
    ui.data_mut(|data| {
        data.insert_temp(
            id_cal_target_size,
            ui.min_rect().width() - this_target_width,
        )
    });

    // or in terms of overflow(-) / underflow(+):
    // next frame target width
    //   == this frame target width + this frame over(-)/under(+)flow
    // this frame over(-)/under(+)flow
    //   == this frame initial max rect width - this frame final min rect width
    return res;
}

pub fn height<R>(ui: &mut eframe::egui::Ui, add_contents: impl FnOnce(&mut Ui, f32) -> R) -> R {
    const HEIGHT_ID: &str = ID!("cal_target_height");
    // this frame target height
    //   == this frame initial max rect height - last frame others height
    let id_cal_target_size = Id::new(HEIGHT_ID);
    let this_init_max_height = dbg!(ui.available_height());
    let last_others_height = ui.data(|data| {
        data.get_temp(id_cal_target_size)
            .unwrap_or(this_init_max_height)
    });
    // this is the total available space for expandable widgets, you can divide
    // it up if you have multiple widgets to expand, even with different ratios.
    let this_target_height = dbg!(this_init_max_height - last_others_height);
    let res = add_contents(ui, this_target_height);

    // this frame others height
    //   == this frame final min rect height - this frame target height
    ui.data_mut(|data| {
        data.insert_temp(
            id_cal_target_size,
            ui.min_rect().height() - this_target_height,
        )
    });

    // or in terms of overflow(-) / underflow(+):
    // next frame target height
    //   == this frame target height + this frame over(-)/under(+)flow
    // this frame over(-)/under(+)flow
    //   == this frame initial max rect height - this frame final min rect height
    return res;
}
