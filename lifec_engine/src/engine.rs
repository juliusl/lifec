/// Struct containing engine config/state,
/// 
/// # Background
/// 
/// By definition an engine is a sequence of event. This struct will be built by defining events and sequencing in a seperate file using runmd.
/// 
/// Events will be configured via a plugin model. Plugins will execute when the event is loaded in the order they are defined. 
/// 
/// Plugins are executed as "Thunks" in a "call-by-name" fashion. Plugins belonging to an event share state linearly, meaning after a plugin executes, it can modify state before the next plugin executes.
/// 
/// An event may have 1 or more plugins.
/// 
pub struct Engine {

}