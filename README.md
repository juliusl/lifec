# Lifec Runtime

Lifec is a runtime based on `runmd` and `specs ECS`. It uses the `reality` parser to compile `runmd` into "block" data. These blocks are then interpreted by plugins that are installed w/ the runtime in order to setup the specs World before any execution happens. An engine is a sequence of events, and lifec provides custom attributes to define engines within runmd. Lifec also provides a specs system that can drive these sequences after interpretation completes.

To interact with lifec, consumers implement a Project trait, and use the Host struct to perform these actions. The main output of this process is a specs World. The host can return a dispatcher builder with default lifec systems, but this also allows for customization.

This is the extent of what lifec provides. Since the Host can be consumed into a World, implementation afterwards is decided by the consumer of the library.

## Starting an engine

An engine is declared within a control block. For example,

```md
<``` control>
+ .engine
: .event setup
: .event install
: .exit
<```>

<``` setup control>
+ .runtime
: .process apt-get update
: .process apt-get upgrade
<```>

<``` install control>
+ .runtime
: .stop_on_error
: .process apt-get install jq, curl

<```>
```

*Note - The syntax `<``` setup control>` is just a way to escape the blocks within markdown. But in the normal case the `<>` are not required in either markdown or runmd.

With a host, you can parse the above, and start this engine by name, i.e `host.start("control")`.

## Engine lifecycle

Once an engine begins, it will execute all events in sequence. (Note that an event is also a sequence of plugin calls). Afterwards, there are several options for what happens next.

* `.fork <control>, <control>` - Sets the engine to start a list of engines

* `.next <control>` - Sets the engine to start another engine

* `.next <event>` - Sets the engine to start another event

* `.repeat <count>` - Set the engine to repeat for count, if unset this will repeat forever

* `.exit`           - Sets the engine to signal the host to exit the process

If no option is used, then runtime will not do anything else. This leaves it up to the programmer to decide what to do next.

## Advanced lifecycle options

* `.once` - Instead of using `.event`, use `.once` w/ the `.repeat` option so that the next loop will skip over these events

* `.fix` - If a runtime is set to `.stop_on_error`, registering a `.fix` within an event runtime can be declared to attempt to fix the stopped sequence.  

* `.event <name> <symbol>` - Passing two identifiers will link in an event defined in a different control namespace. This is advanced because it creates a dependency between two control domains, but overall this can sometimes be desired behavior. Especially when prototyping.

## Plugin Development
TODO

## Advanced Plugin Development
TODO

