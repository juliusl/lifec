use clap::Args;


/// Struct representing an order engine that will be received by the runner,
/// 
#[derive(Default, Args)]
pub struct Order {
    /// The name of the control block for this engine,
    ///  
    #[clap(long, short, default_value_t = String::from("order"))]
    control_symbol: String,
    /// The name of the event that will accept this order and reply to the sender,
    /// 
    #[clap(long, short, default_value_t = String::from("accept"))]
    accept_symbol: String,
}

