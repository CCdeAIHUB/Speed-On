use std::env;
use std::io::{self, BufReader};

use speed_on_ipc_stdio::{
    open_command_opener_dispatcher, open_default_dispatcher, run_json_lines_transport, StdioConfig,
};

fn main() {
    if let Err(error) = run() {
        let encoded = match serde_json::to_string(&error) {
            Ok(value) => value,
            Err(_) => String::from(
                "{\"ok\":false,\"error_code\":\"IPC_STDIO_IO_FAILURE\",\"message\":\"failed to encode startup error\",\"module\":\"ipc_stdio::main\",\"recoverable\":false}",
            ),
        };
        eprintln!("{encoded}");
        std::process::exit(1);
    }
}

fn run() -> speed_on_ipc_stdio::StdioResult<()> {
    let config = StdioConfig::from_args_and_env(env::args().skip(1), env::var("SPEED_ON_DB").ok())?;
    let stdin = io::stdin();
    let stdout = io::stdout();

    if config.enable_command_opener {
        let mut dispatcher = open_command_opener_dispatcher(&config)?;
        run_json_lines_transport(BufReader::new(stdin.lock()), stdout.lock(), &mut dispatcher)
    } else {
        let mut dispatcher = open_default_dispatcher(&config)?;
        run_json_lines_transport(BufReader::new(stdin.lock()), stdout.lock(), &mut dispatcher)
    }
}
