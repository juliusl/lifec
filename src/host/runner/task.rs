/// This struct represents the "Task" engine returned by the runner, once execution of an operation has started,
/// 
#[derive(Default, Args)]
pub struct Task {
    /// Name of the control block of this engine,
    /// 
    #[clap(short, long, default_value_t = String::from("task"))]
    control_symbol: String,
    /// Name of the event that can monitor the operation,
    /// 
    #[clap(long, default_value_t = String::from("monitor"))]
    monitor_symbol: String,
    /// Name of the event that can cancel the operation, 
    /// 
    #[clap(long, default_value_t = String::from("cancel"))]
    cancel_symbol: String,
}
