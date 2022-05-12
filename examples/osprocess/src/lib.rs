use std::{process::{Command, Output}, fmt::Display};

use lifec::RuntimeState;

#[derive(Debug, Clone, Default)]
pub struct Process {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: i32,
}

impl Display for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "process")?;

        Ok(())
    }
}

pub struct ProcessExecutionError;

impl RuntimeState for Process {
    type Error = ProcessExecutionError;
    type State = Process;

    fn load<'a, S: AsRef<str> + ?Sized>(&self, init: &'a S) -> Self
    where
        Self: Sized,
    {
        todo!()
    }

    fn process<'a, S: AsRef<str> + ?Sized>(&self, msg: &'a S) -> Result<Self::State, Self::Error> {
        let command = Command::new(msg.as_ref()).output();

        let output = command.ok();
        if let Some(Output { status, stdout, stderr }) = output {
            let process = Process { stdout, stderr, code: status.code().unwrap_or(-1) };
        
            Ok(process)
        } else {
            Err(ProcessExecutionError{})
        }
    }

    fn process_with_args<S: AsRef<str> + ?Sized>(
        state: lifec::WithArgs<Self>,
        msg: &S,
    ) -> Result<Self::State, Self::Error>
    where
        Self: Clone + Default + RuntimeState<State = Self>,
    {
        let command = Command::new(msg.as_ref()).args(state.get_args()).output();

        let output = command.ok();
        if let Some(Output { status, stdout, stderr }) = output {
            let process = Process { stdout, stderr, code: status.code().unwrap_or(-1) };
        
            Ok(process)
        } else {
            Err(ProcessExecutionError{})
        }
    }
}
