use clap::Args;

/// Menu is a struct representing a "menu" engine that is provided by a runner,
/// 
#[derive(Default, Args)]
pub struct Menu {
    /// Name of the control block for this engine,
    /// 
    #[clap(short, long, default_value_t = String::from("menu"))]
    control_symbol: String,
    /// The name of the event that can send an order to the runner,
    /// 
    #[clap(short, long, default_value_t = String::from("send"))]
    send_name: String, 
}

